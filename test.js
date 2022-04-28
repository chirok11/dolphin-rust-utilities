const api = require('./');
const path = require('path');
const { EventEmitter } = require('events');

(async() => {
    api.rustLoggerInit();
    console.info(path.join(__dirname, 'data_dir'));
})();