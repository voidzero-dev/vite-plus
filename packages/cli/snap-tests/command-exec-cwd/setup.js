const fs = require('fs');
// Create a subdirectory to test cwd preservation
fs.mkdirSync('src/nested', { recursive: true });
