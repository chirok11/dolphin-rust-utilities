const api = require('./');
const path = require('path');

(async() => {
    api.rustLoggerInit();
    api.killProcessByPid(52877);
})();