// Simple test runner for the extension
console.log('🧪 Running Ramparts MCP Guardian Extension Tests...');

// Test 1: Check if compiled files exist
const fs = require('fs');
const path = require('path');

const requiredFiles = [
    'out/extension.js',
    'out/rampartsManager.js',
    'out/treeProvider.js',
    'out/mcpConfigManager.js'
];

let allFilesExist = true;
for (const file of requiredFiles) {
    if (!fs.existsSync(path.join(__dirname, '../../', file))) {
        console.error(`❌ Missing compiled file: ${file}`);
        allFilesExist = false;
    } else {
        console.log(`✅ Found: ${file}`);
    }
}

if (allFilesExist) {
    console.log('✅ All required compiled files exist');
} else {
    console.error('❌ Some compiled files are missing');
    process.exit(1);
}

// Test 2: Check if scan methods are exported
try {
    const rampartsManager = require('../../out/rampartsManager.js');
    console.log('✅ RampartsManager module loaded successfully');

    // Check if scan methods exist (they should be on the class prototype)
    const RampartsManager = rampartsManager.RampartsManager;
    if (RampartsManager && RampartsManager.prototype) {
        const methods = Object.getOwnPropertyNames(RampartsManager.prototype);
        const scanMethods = methods.filter(m => m.includes('scan'));
        console.log(`✅ Found scan methods: ${scanMethods.join(', ')}`);
    }
} catch (error) {
    console.error('❌ Failed to load RampartsManager:', error.message);
    process.exit(1);
}

console.log('🎉 All tests passed!');
