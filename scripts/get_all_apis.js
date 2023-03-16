const fs = require('fs');
const path = require('path');

async function __main__() {
  const root = path.join(__dirname, '../crates/tauri_lite/src/apis');
  const apis = fs.readdirSync(root).filter(f => f !== 'mod.rs').map(file => {
    const content = fs.readFileSync(path.join(root, file), 'utf8');

    const apis = content.match(/\"(\w+\.\w+)\"/mg).map(m => m.replace(/\"/g, ''));

    for (const api of apis) {
      console.log(`['${api.split('.').pop()}', <div>${api}</div>],`);
    }

    return [file, apis];
  });
}

__main__();