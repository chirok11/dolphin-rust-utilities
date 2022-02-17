const api = require('./');
const path = require('path');

(async() => {
    api.loggerInit();
    api.setForegroundByPid(54679)
})();