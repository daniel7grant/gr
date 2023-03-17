#!/usr/bin/env node
const { spawn } = require('child_process');

spawn('./gr', process.argv.slice(2), { stdio: 'inherit' });
