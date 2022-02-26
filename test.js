const api = require('./');
const path = require('path');

(async() => {
    api.rustLoggerInit();
    console.info(path.join(__dirname, 'data_dir'));
    api.archivateFolder('browser.zip', path.join(__dirname, 'data_dir'), 
    [
        'Default/Extensions/**/*'
    ]);
})();