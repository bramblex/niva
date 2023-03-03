const sharp = require('sharp');
const path = require('path');
const fs = require('fs');

async function __main__() {
  const inputPath = path.resolve(__dirname, '../src/assets/logo.png');
  const outputPath = path.resolve(__dirname, '../src/assets/icon.bitmap');

  const buffer = await sharp(inputPath)
    .resize(32, 32)
    .raw()
    .toBuffer();
  
  fs.writeFileSync(outputPath, buffer);
}

__main__();
