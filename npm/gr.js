#!/usr/bin/env node
const { spawn } = require('child_process');
const path = require('path');

spawn(path.join(__dirname, 'gr'), process.argv.slice(2), { stdio: 'inherit' });
