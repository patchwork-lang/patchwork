#!/usr/bin/env node
/**
 * Patchwork Code Process Initialization Script
 *
 * This script is spawned by the prompt process (Claude agent) to run compiled
 * Patchwork worker code. It sets up the bidirectional IPC channel via stdio
 * and executes the worker's main function.
 *
 * Usage:
 *   node code-process-init.js <worker_name> <session_json>
 *
 * Arguments:
 *   worker_name - Name of the worker to execute (e.g., 'greeter', 'analyst')
 *   session_json - JSON string with session context (id, timestamp, workDir)
 *
 * The script expects the compiled worker module to be at:
 *   ./workers/<worker_name>.js
 *
 * Session JSON format:
 * {
 *   "sessionId": "historian-20251024-120316",
 *   "timestamp": "2025-10-24T12:03:16Z",
 *   "workDir": "/tmp/historian-20251024-120316",
 *   "params": { ... }  // Worker-specific parameters
 * }
 */

import { SessionContext, getStdinReader } from './patchwork-runtime.js';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

// Get current directory (for ES modules)
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

/**
 * Parse command line arguments
 */
function parseArgs() {
  const args = process.argv.slice(2);

  if (args.length < 1) {
    console.error('Usage: node code-process-init.js <worker_name> [session_json]');
    console.error('');
    console.error('Session JSON can also be provided via stdin.');
    process.exit(1);
  }

  return {
    workerName: args[0],
    sessionJson: args[1]  // May be undefined - will read from stdin
  };
}

/**
 * Read session context from stdin or command line
 */
async function readSessionContext(sessionJson) {
  let sessionData;

  if (sessionJson) {
    // Session provided as command line argument
    try {
      sessionData = JSON.parse(sessionJson);
    } catch (err) {
      console.error('[Code Process] Failed to parse session JSON from command line:', err);
      process.exit(1);
    }
  } else {
    // Read first line from stdin using the runtime's StdinReader
    // This ensures the same reader is used for both session init and IPC
    try {
      const reader = getStdinReader();
      sessionData = await reader.readMessage();
    } catch (err) {
      console.error('[Code Process] Failed to read session JSON from stdin:', err);
      process.exit(1);
    }
  }

  return sessionData;
}

/**
 * Main entry point
 */
async function main() {
  try {
    // Parse arguments
    const { workerName, sessionJson } = parseArgs();

    // Read session context
    const sessionData = await readSessionContext(sessionJson);

    // Create session context
    const session = new SessionContext(
      sessionData.sessionId,
      sessionData.timestamp,
      sessionData.workDir
    );

    console.error(`[Code Process] Starting worker: ${workerName}`);
    console.error(`[Code Process] Session ID: ${session.id}`);
    console.error(`[Code Process] Work directory: ${session.dir}`);

    // Dynamically import the compiled module
    const modulePath = resolve(__dirname, 'index.js');
    const compiledModule = await import(modulePath);

    // Check if the function exists in the module
    if (typeof compiledModule[workerName] !== 'function') {
      throw new Error(`Compiled module does not export a function named '${workerName}'`);
    }

    const workerFunction = compiledModule[workerName];

    // Extract worker parameters from session data
    const params = sessionData.params || {};
    const paramValues = Object.values(params);

    // Execute the worker function with session context and parameters
    console.error(`[Code Process] Executing worker function...`);
    const result = await workerFunction(session, ...paramValues);

    // Send result back to prompt process via stdout
    const response = {
      type: 'workerComplete',
      worker: workerName,
      result: result
    };

    process.stdout.write(JSON.stringify(response) + '\n');

    console.error(`[Code Process] Worker completed successfully`);

    // Exit successfully
    process.exit(0);
  } catch (error) {
    console.error(`[Code Process] Worker failed:`, error);

    // Send error back to prompt process
    const errorResponse = {
      type: 'error',
      message: error.message,
      stack: error.stack
    };

    process.stdout.write(JSON.stringify(errorResponse) + '\n');

    // Exit with error code
    process.exit(1);
  }
}

// Run main function
main();
