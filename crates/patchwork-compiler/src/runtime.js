/**
 * Patchwork Runtime Library
 *
 * Provides the runtime infrastructure for compiled Patchwork programs.
 * This file is automatically emitted by the Patchwork compiler.
 */

import { spawn } from 'child_process';
import { promisify } from 'util';

/**
 * Mailbox for worker message passing (Phase 5)
 *
 * Provides FIFO message queue with blocking receive.
 */
export class Mailbox {
  constructor(name) {
    this.name = name;
    this.queue = [];
    this.waiters = [];
  }

  /**
   * Send a message to this mailbox
   *
   * @param {any} message - The message to send (will be JSON serialized)
   */
  send(message) {
    // Clone the message to ensure isolation between workers
    const cloned = JSON.parse(JSON.stringify(message));

    // If there's a waiter, resolve it immediately
    if (this.waiters.length > 0) {
      const waiter = this.waiters.shift();
      clearTimeout(waiter.timeoutId);
      waiter.resolve(cloned);
    } else {
      // Otherwise, queue the message
      this.queue.push(cloned);
    }
  }

  /**
   * Receive a message from this mailbox
   *
   * Blocks until a message is available or timeout is reached.
   *
   * @param {number} timeout - Timeout in milliseconds (optional)
   * @returns {Promise<any>} - The received message
   * @throws {Error} - If timeout is reached before a message arrives
   */
  async receive(timeout) {
    // If there's a queued message, return it immediately
    if (this.queue.length > 0) {
      return this.queue.shift();
    }

    // Otherwise, wait for a message
    return new Promise((resolve, reject) => {
      const waiter = { resolve, reject, timeoutId: null };

      if (timeout !== undefined && timeout !== null) {
        waiter.timeoutId = setTimeout(() => {
          // Remove this waiter from the list
          const index = this.waiters.indexOf(waiter);
          if (index !== -1) {
            this.waiters.splice(index, 1);
          }
          reject(new Error(`Mailbox receive timeout after ${timeout}ms`));
        }, timeout);
      }

      this.waiters.push(waiter);
    });
  }
}

/**
 * Mailroom manages all mailboxes for a session (Phase 5)
 *
 * Provides lazy mailbox creation via property access.
 */
export class Mailroom {
  constructor() {
    this.mailboxes = new Map();

    // Return a proxy that creates mailboxes on-demand
    return new Proxy(this, {
      get(target, prop) {
        // Allow access to internal methods/properties
        if (prop === 'mailboxes' || typeof target[prop] === 'function') {
          return target[prop];
        }

        // Lazy mailbox creation
        if (!target.mailboxes.has(prop)) {
          target.mailboxes.set(prop, new Mailbox(prop));
        }
        return target.mailboxes.get(prop);
      }
    });
  }
}

/**
 * Session context available to all workers
 */
export class SessionContext {
  constructor(id, timestamp, dir) {
    this.id = id;
    this.timestamp = timestamp;
    this.dir = dir;
    // Phase 5: Add mailroom for message passing
    this.mailbox = new Mailroom();
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

/**
 * IPC Message types for prompt execution
 * (Phase 3: scaffolding only, full implementation in Phase 11)
 */
export class IpcMessage {
  constructor(type, data) {
    this.type = type;
    this.data = data;
  }
}

export class ThinkRequest extends IpcMessage {
  constructor(templateId, bindings) {
    super('ThinkRequest', { templateId, bindings });
  }
}

export class ThinkResponse extends IpcMessage {
  constructor(result) {
    super('ThinkResponse', { result });
  }
}

export class AskRequest extends IpcMessage {
  constructor(templateId, bindings) {
    super('AskRequest', { templateId, bindings });
  }
}

export class AskResponse extends IpcMessage {
  constructor(result) {
    super('AskResponse', { result });
  }
}

/**
 * Execute a prompt block (think or ask)
 *
 * Phase 4: Sends IPC request with template ID and variable bindings.
 * Phase 11: Full IPC implementation with actual agent communication.
 *
 * @param {SessionContext} session - The session context
 * @param {string} templateId - The prompt template ID (e.g., 'think_0')
 * @param {Object} bindings - Variable bindings to interpolate into the template
 * @returns {Promise<any>} - The result from the agent (structure depends on prompt type)
 */
export async function executePrompt(session, templateId, bindings) {
  // Phase 4: Mock implementation that just returns a placeholder
  // Phase 11 will implement the full IPC transport

  console.log(`[Patchwork Runtime] executePrompt: ${templateId}`);
  console.log(`[Patchwork Runtime] Session: ${session.id}`);
  console.log(`[Patchwork Runtime] Bindings:`, bindings);

  // Return a mock response for now
  // In Phase 11, this will send an IPC message and await the response
  return {
    success: true,
    message: `Mock response for ${templateId}`,
  };
}
