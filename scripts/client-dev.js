#!/usr/bin/env node
const { existsSync } = require('fs');
const { spawnSync } = require('child_process');
const path = require('path');

const repoRoot = path.resolve(__dirname, '..');
const clientDir = path.join(repoRoot, 'client');
const nodeModules = path.join(clientDir, 'node_modules');

function run(cmd, args) {
  const r = spawnSync(cmd, args, { cwd: clientDir, stdio: 'inherit', shell: true });
  if (r.error) {
    console.error(r.error);
    process.exit(1);
  }
  if (typeof r.status === 'number' && r.status !== 0) process.exit(r.status);
}

if (!existsSync(nodeModules)) {
  console.log('client/node_modules not found — running npm install in client...');
  run('npm', ['install']);
}

console.log('Starting client dev server...');
run('npm', ['run', 'dev']);
