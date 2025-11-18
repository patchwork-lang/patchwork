/**
 * Patchwork Runtime Library
 *
 * Provides the runtime infrastructure for compiled Patchwork programs.
 * This file is automatically emitted by the Patchwork compiler.
 */

import { spawn } from 'child_process';
import { promisify } from 'util';
import { watch } from 'fs';
import { writeFile, readFile, access, readdir, unlink, mkdir } from 'fs/promises';

/**
 * Mailbox for worker message passing
 *
 * Provides FIFO message queue with blocking receive using filesystem-based storage.
 * Messages are stored as individual files in session.dir/mailboxes/{name}/ to avoid
 * race conditions when multiple processes write simultaneously.
 *
 * Integrates with session failure detection to abort on worker failures.
 */
export class Mailbox {
  constructor(name, session) {
    this.name = name;
    this.session = session;
    this.mailboxDir = `${session.dir}/mailboxes/${name}`;
  }

  /**
   * Ensure mailbox directory exists
   */
  async _ensureDir() {
    try {
      await mkdir(this.mailboxDir, { recursive: true });
    } catch (err) {
      // Directory might already exist, ignore EEXIST errors
      if (err.code !== 'EEXIST') {
        throw err;
      }
    }
  }

  /**
   * Send a message to this mailbox
   *
   * Creates a new message file with timestamp-PID naming to ensure uniqueness
   * and FIFO ordering across processes.
   *
   * @param {any} message - The message to send (will be JSON serialized)
   * @throws {Error} - If the session has failed
   */
  async send(message) {
    // Check if session has failed before sending
    await this.session.checkFailed();

    // Ensure mailbox directory exists
    await this._ensureDir();

    // Create unique filename: timestamp_ms-pid.json
    const filename = `${Date.now()}-${process.pid}.json`;
    const filepath = `${this.mailboxDir}/${filename}`;

    // Create message envelope with metadata
    const envelope = {
      from: process.pid.toString(),  // Sender process ID
      to: this.name,
      timestamp: new Date().toISOString(),
      payload: message
    };

    // Write atomically - file creation is atomic operation
    await writeFile(filepath, JSON.stringify(envelope, null, 2));
  }

  /**
   * Receive a message from this mailbox
   *
   * Blocks until a message is available, session fails, or timeout is reached.
   * Reads oldest message file (by lexicographic filename sort) and deletes it.
   *
   * @param {number} timeout - Timeout in milliseconds (optional)
   * @returns {Promise<any>} - The received message payload
   * @throws {Error} - If timeout is reached or session fails before a message arrives
   */
  async receive(timeout) {
    // Check if session has already failed
    await this.session.checkFailed();

    // Ensure mailbox directory exists
    await this._ensureDir();

    // Try to read an existing message
    const message = await this._tryReadMessage();
    if (message !== null) {
      return message;
    }

    // No messages yet - watch for new files
    return this._watchForMessage(timeout);
  }

  /**
   * Try to read the oldest message file if one exists
   *
   * @returns {Promise<any|null>} - Message payload or null if no messages
   */
  async _tryReadMessage() {
    try {
      // List all files in mailbox directory
      const files = await readdir(this.mailboxDir);

      // Filter out non-message files and sort lexicographically (FIFO order)
      const messageFiles = files
        .filter(f => f.endsWith('.json'))
        .sort();

      if (messageFiles.length === 0) {
        return null;
      }

      // Read oldest message
      const filepath = `${this.mailboxDir}/${messageFiles[0]}`;
      const content = await readFile(filepath, 'utf-8');

      // Delete the file after reading
      await unlink(filepath);

      // Parse envelope and return payload
      const envelope = JSON.parse(content);
      return envelope.payload;
    } catch (err) {
      // If file was deleted by another receiver, return null
      if (err.code === 'ENOENT') {
        return null;
      }
      throw err;
    }
  }

  /**
   * Watch for new message files in the mailbox directory
   *
   * @param {number} timeout - Timeout in milliseconds (optional)
   * @returns {Promise<any>} - The received message payload
   * @throws {Error} - If timeout is reached or session fails
   */
  async _watchForMessage(timeout) {
    return new Promise((resolve, reject) => {
      let watcher = null;
      let timeoutId = null;
      let checkIntervalId = null;

      const cleanup = () => {
        if (watcher) {
          watcher.close();
          watcher = null;
        }
        if (timeoutId) {
          clearTimeout(timeoutId);
          timeoutId = null;
        }
        if (checkIntervalId) {
          clearInterval(checkIntervalId);
          checkIntervalId = null;
        }
      };

      // Watch for file system events
      watcher = watch(this.mailboxDir, async (eventType, filename) => {
        if (eventType === 'rename' && filename && filename.endsWith('.json')) {
          // New file created - try to read it
          const message = await this._tryReadMessage();
          if (message !== null) {
            cleanup();
            resolve(message);
          }
        }
      });

      // Also poll periodically in case fs.watch misses events
      // (fs.watch can be unreliable on some systems)
      checkIntervalId = setInterval(async () => {
        const message = await this._tryReadMessage();
        if (message !== null) {
          cleanup();
          resolve(message);
        }
      }, 100);  // Check every 100ms

      // Set up timeout
      if (timeout !== undefined && timeout !== null) {
        timeoutId = setTimeout(() => {
          cleanup();
          reject(new Error(`Mailbox receive timeout after ${timeout}ms`));
        }, timeout);
      }

      // Race against session failure
      this.session.failurePromise.catch(err => {
        cleanup();
        reject(err);
      });
    });
  }
}

/**
 * Mailroom manages all mailboxes for a session
 *
 * Provides lazy mailbox creation via property access.
 */
export class Mailroom {
  constructor(session) {
    this.session = session;
    this.mailboxes = new Map();

    // Return a proxy that creates mailboxes on-demand
    return new Proxy(this, {
      get(target, prop) {
        // Allow access to internal methods/properties
        if (prop === 'session' || prop === 'mailboxes' || typeof target[prop] === 'function') {
          return target[prop];
        }

        // Lazy mailbox creation
        if (!target.mailboxes.has(prop)) {
          target.mailboxes.set(prop, new Mailbox(prop, target.session));
        }
        return target.mailboxes.get(prop);
      }
    });
  }
}

/**
 * Session context available to all workers
 *
 * Provides session state and coordinates failure detection across workers.
 * Uses filesystem-based failure tracking that works across processes.
 */
export class SessionContext {
  constructor(id, timestamp, dir) {
    this.id = id;
    this.timestamp = timestamp;
    this.dir = dir;
    this.failureFile = `${dir}/.failed`;
    this.failureWatcher = null;
    this.failurePromise = null;

    // Mailroom for worker message passing
    this.mailbox = new Mailroom(this);

    // Set up failure detection
    this.setupFailureWatch();
  }

  /**
   * Set up filesystem watcher for session failure detection
   *
   * Creates a promise that rejects when .failed file is created.
   * This allows mailbox operations to race against session failure.
   */
  setupFailureWatch() {
    this.failurePromise = new Promise((_resolve, reject) => {
      // First check if .failed already exists (session may have failed before we joined)
      access(this.failureFile)
        .then(() => {
          // File exists - session already failed
          return readFile(this.failureFile, 'utf-8');
        })
        .then(content => {
          const failureInfo = JSON.parse(content);
          reject(new Error(`Session ${this.id} failed: ${failureInfo.error}`));
        })
        .catch(err => {
          // File doesn't exist yet - set up watcher
          if (err.code !== 'ENOENT') {
            // Some other error reading the file
            console.error('Error checking failure file:', err);
          }

          // Watch the session directory for .failed file creation
          this.failureWatcher = watch(this.dir, (_eventType, filename) => {
            if (filename === '.failed') {
              readFile(this.failureFile, 'utf-8')
                .then(content => {
                  const failureInfo = JSON.parse(content);
                  reject(new Error(`Session ${this.id} failed: ${failureInfo.error}`));
                })
                .catch(readErr => {
                  reject(new Error(`Session ${this.id} failed but could not read error details: ${readErr.message}`));
                });
            }
          });
        });
    });
  }

  /**
   * Mark this session as failed
   *
   * Writes .failed file to notify all workers in this session.
   * This is called when a worker throws an error.
   *
   * @param {Error} error - The error that caused the failure
   */
  async markFailed(error) {
    const failureInfo = {
      timestamp: new Date().toISOString(),
      error: error.message,
      stack: error.stack
    };

    try {
      await writeFile(this.failureFile, JSON.stringify(failureInfo, null, 2));
    } catch (writeErr) {
      // If we can't write the failure file, log it
      console.error('Failed to write session failure file:', writeErr);
    }
  }

  /**
   * Check if session has failed
   *
   * Checks for existence of .failed file synchronously.
   * Throws if session has failed.
   *
   * @throws {Error} - If the session has failed
   */
  async checkFailed() {
    try {
      // Check if .failed file exists
      await access(this.failureFile);

      // File exists - read it and throw
      const content = await readFile(this.failureFile, 'utf-8');
      const failureInfo = JSON.parse(content);
      throw new Error(`Session ${this.id} failed: ${failureInfo.error}`);
    } catch (err) {
      // If error is about session failure, re-throw it
      if (err.message && err.message.includes('Session')) {
        throw err;
      }
      // Otherwise it's just ENOENT (file doesn't exist) - session is fine
    }
  }

  /**
   * Clean up resources
   *
   * Should be called when session completes (success or failure).
   */
  cleanup() {
    if (this.failureWatcher) {
      this.failureWatcher.close();
      this.failureWatcher = null;
    }
  }
}

/**
 * Shell command execution options
 */
class ShellOptions {
  constructor(options = {}) {
    this.capture = options.capture ?? false;
    this.cwd = options.cwd ?? process.cwd();
  }
}

/**
 * Execute a shell command
 *
 * @param {string} command - The command string to execute
 * @param {Object} options - Execution options
 * @param {boolean} options.capture - Whether to capture and return stdout
 * @param {string} options.cwd - Working directory for command execution
 * @returns {Promise<string>} - The stdout output if capture=true, otherwise empty string
 */
export async function shell(command, options = {}) {
  const opts = new ShellOptions(options);

  return new Promise((resolve, reject) => {
    const child = spawn('sh', ['-c', command], {
      cwd: opts.cwd,
      stdio: opts.capture ? ['ignore', 'pipe', 'pipe'] : ['ignore', 'inherit', 'inherit']
    });

    if (opts.capture) {
      let stdout = '';
      let stderr = '';

      child.stdout.on('data', (data) => {
        stdout += data.toString();
      });

      child.stderr.on('data', (data) => {
        stderr += data.toString();
      });

      child.on('close', (code) => {
        if (code !== 0) {
          reject(new Error(`Command failed with exit code ${code}: ${stderr}`));
        } else {
          resolve(stdout.trimEnd());
        }
      });

      child.on('error', (err) => {
        reject(err);
      });
    } else {
      child.on('close', (code) => {
        if (code !== 0) {
          reject(new Error(`Command failed with exit code ${code}`));
        } else {
          resolve('');
        }
      });

      child.on('error', (err) => {
        reject(err);
      });
    }
  });
}

// Export as $shell for generated code
export { shell as $shell };

/**
 * Execute a shell pipe (cmd1 | cmd2)
 *
 * Connects stdout of first command to stdin of second command.
 *
 * @param {Array<string>} commands - Array of command strings to pipe together
 * @param {Object} options - Execution options
 * @returns {Promise<string>} - The output of the final command if capture=true
 */
export async function $shellPipe(commands, options = {}) {
  // Join commands with pipe operator and execute as single shell command
  const pipeCmd = commands.join(' | ');
  return shell(pipeCmd, options);
}

/**
 * Execute shell commands with && operator (cmd1 && cmd2)
 *
 * Executes second command only if first succeeds.
 *
 * @param {Array<string>} commands - Array of command strings to chain
 * @param {Object} options - Execution options
 * @returns {Promise<string>} - The output if capture=true, otherwise empty string
 */
export async function $shellAnd(commands, options = {}) {
  const andCmd = commands.join(' && ');
  return shell(andCmd, options);
}

/**
 * Execute shell commands with || operator (cmd1 || cmd2)
 *
 * Executes second command only if first fails.
 *
 * @param {Array<string>} commands - Array of command strings to chain
 * @param {Object} options - Execution options
 * @returns {Promise<string>} - The output if capture=true, otherwise empty string
 */
export async function $shellOr(commands, options = {}) {
  const orCmd = commands.join(' || ');
  return shell(orCmd, options);
}

/**
 * Execute shell command with redirection (cmd > file)
 *
 * @param {string} command - The command string to execute
 * @param {string} operator - The redirection operator ('>', '>>', '<', '2>', '2>&1')
 * @param {string} target - The file path or descriptor for redirection
 * @param {Object} options - Execution options
 * @returns {Promise<string>} - Empty string (redirections don't capture)
 */
export async function $shellRedirect(command, operator, target, options = {}) {
  // Build the full command with redirection
  const redirectCmd = `${command} ${operator} ${target}`;
  return shell(redirectCmd, { ...options, capture: false });
}

/**
 * Stdin reading helper for IPC
 *
 * Reads newline-delimited JSON messages from stdin.
 * Used for bidirectional communication with prompt process.
 */
class StdinReader {
  constructor() {
    this.buffer = '';
    this.waiters = [];
    this.setupStdin();
  }

  setupStdin() {
    // Set stdin to read in 'utf8' mode
    process.stdin.setEncoding('utf8');

    // Read data from stdin
    process.stdin.on('data', (chunk) => {
      this.buffer += chunk;
      this.processBuffer();
    });

    // Handle stdin close
    process.stdin.on('end', () => {
      // Reject all pending waiters
      for (const waiter of this.waiters) {
        waiter.reject(new Error('stdin closed unexpectedly'));
      }
      this.waiters = [];
    });
  }

  processBuffer() {
    // Process all complete lines in the buffer
    while (true) {
      const newlineIndex = this.buffer.indexOf('\n');
      if (newlineIndex === -1) {
        // No complete line yet
        break;
      }

      // Extract the line
      const line = this.buffer.slice(0, newlineIndex);
      this.buffer = this.buffer.slice(newlineIndex + 1);

      // Skip empty lines
      if (line.trim() === '') {
        continue;
      }

      // Parse JSON and deliver to first waiter
      try {
        const message = JSON.parse(line);
        if (this.waiters.length > 0) {
          const waiter = this.waiters.shift();
          waiter.resolve(message);
        } else {
          // No one waiting - this shouldn't happen in normal operation
          console.error('[Patchwork Runtime] Received message but no one waiting:', message);
        }
      } catch (err) {
        console.error('[Patchwork Runtime] Failed to parse IPC message:', line, err);
      }
    }
  }

  /**
   * Read the next message from stdin
   *
   * @param {number} timeout - Optional timeout in milliseconds
   * @returns {Promise<Object>} - The parsed JSON message
   * @throws {Error} - If timeout is reached or stdin closes
   */
  async readMessage(timeout) {
    return new Promise((resolve, reject) => {
      // Add to waiters queue
      const waiter = { resolve, reject };
      this.waiters.push(waiter);

      // Set up timeout if provided
      if (timeout !== undefined && timeout !== null) {
        setTimeout(() => {
          // Remove from waiters if still present
          const index = this.waiters.indexOf(waiter);
          if (index !== -1) {
            this.waiters.splice(index, 1);
            reject(new Error(`IPC read timeout after ${timeout}ms`));
          }
        }, timeout);
      }
    });
  }
}

// Global stdin reader instance
let stdinReader = null;

/**
 * Get or create the global stdin reader
 *
 * @returns {StdinReader} - The global stdin reader instance
 */
export function getStdinReader() {
  if (!stdinReader) {
    stdinReader = new StdinReader();
  }
  return stdinReader;
}

/**
 * Send an IPC message to the prompt process via stdout
 *
 * @param {Object} message - The message to send (will be JSON serialized)
 */
function sendIpcMessage(message) {
  const line = JSON.stringify(message);
  process.stdout.write(line + '\n');
}

/**
 * Execute a prompt block (think or ask)
 *
 * Sends IPC request with skill name and variable bindings to the prompt process.
 * Blocks until the prompt process sends back a response.
 *
 * @param {SessionContext} session - The session context
 * @param {string} skillName - The skill name (e.g., 'greeter_think_0')
 * @param {Object} bindings - Variable bindings to interpolate into the prompt
 * @returns {Promise<any>} - The result from the agent
 * @throws {Error} - If the prompt execution fails or times out
 */
export async function executePrompt(session, skillName, bindings) {
  // Send IPC request to prompt process
  const request = {
    type: 'executePrompt',
    skill: skillName,
    bindings: bindings || {}
  };

  sendIpcMessage(request);

  // Wait for response from prompt process
  const reader = getStdinReader();
  const response = await reader.readMessage();

  // Handle error responses
  if (response.type === 'error') {
    throw new Error(response.message || 'Prompt execution failed');
  }

  // Return the result value
  return response.value;
}

/**
 * Delegate work to a group of workers (fork/join pattern)
 *
 * Sends IPC message to the prompt process to spawn worker subagents via Task tool.
 * Waits for all workers to complete. If any worker fails, the entire session fails.
 *
 * This implements fork/join semantics:
 * - All workers start in parallel (via Promise.all on generated worker functions)
 * - Prompt process spawns each worker as a Task subagent
 * - All workers must succeed for delegation to succeed
 * - If any worker fails, session is marked failed and other workers abort
 *
 * @param {SessionContext} session - The session context
 * @param {Array<Promise>} workers - Array of worker promises to execute
 * @returns {Promise<Array>} - Array of results from each worker (in same order)
 * @throws {Error} - If any worker fails
 */
export async function delegate(session, workers) {
  try {
    // Send IPC message to prompt process to spawn workers
    // Note: The workers array contains promises that will resolve to worker configs
    // We need to await them first to get the actual configs
    const workerConfigs = await Promise.all(workers);

    const request = {
      type: 'delegate',
      sessionId: session.id,
      workDir: session.dir,
      workers: workerConfigs
    };

    sendIpcMessage(request);

    // Wait for response from prompt process
    const reader = getStdinReader();
    const response = await reader.readMessage();

    // Handle error responses
    if (response.type === 'error') {
      throw new Error(response.message || 'Worker delegation failed');
    }

    // Return the results array
    return response.results;
  } catch (error) {
    // One or more workers failed - mark session as failed
    await session.markFailed(error);

    // Re-throw to propagate the error
    throw error;
  } finally {
    // Clean up session resources
    session.cleanup();
  }
}

/**
 * Standard library: Logging
 *
 * Simple logging function for development and debugging.
 * Maps to console.log for now, can be enhanced with session context later.
 */
export function log(...args) {
  console.log(...args);
}
