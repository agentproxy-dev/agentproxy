#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

// Get the directory where the package is installed
const packageDir = path.resolve(__dirname, '..');

// Check if we're in development mode
const isDev = process.env.NODE_ENV === 'development';

// Determine the command to run
const command = isDev ? 'npm run dev' : 'npm run start';

console.log(`Starting MCP Proxy UI...`);
console.log(`Package directory: ${packageDir}`);

// Spawn the npm process
const npmProcess = spawn(command, [], {
  cwd: packageDir,
  stdio: 'inherit',
  shell: true
});

// Handle process events
npmProcess.on('error', (err) => {
  console.error(`Failed to start MCP Proxy UI: ${err.message}`);
  process.exit(1);
});

npmProcess.on('close', (code) => {
  if (code !== 0) {
    console.error(`MCP Proxy UI exited with code ${code}`);
    process.exit(code);
  }
});

// Handle SIGINT (Ctrl+C)
process.on('SIGINT', () => {
  npmProcess.kill('SIGINT');
  process.exit(0);
}); 