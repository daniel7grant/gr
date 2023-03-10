const axios = require('axios');
const os = require('os');
const unzip = require('unzip-stream');

const { repository, version } = require('./package.json');

// Postinstall script: download binary from GitHub releases
async function main() {
    if (os.machine() !== 'x86_64') {
        // ARM is currently not supported :(
        process.stderr.write('Your platform is currently not supported.\n');
        process.exit(1);
    }

    let filename;
    switch (process.platform) {
        case 'cygwin':
        case 'win32':
            // Windows
            filename = 'gr-bin_x86_64-pc-windows-gnu.zip';
            break;
        case 'linux':
            // Linux
            filename = 'gr-bin_x86_64-unknown-linux-gnu.zip';
            break;
        default:
            // MacOS is currently not supported :(
            process.stderr.write('Your platform is currently not supported.\n');
            process.exit(1);
    }

    const url = `${repository.url}/releases/download/v${version}/${filename}`;

    const file = await axios.get(url, { responseType: 'stream' });
    file.data.pipe(unzip.Extract({ path: '.' }));
}

main().catch((err) => {
    console.error(err);
    process.exit(1);
});
