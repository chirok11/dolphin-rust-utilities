const api = require('./');
const path = require('path');

(async() => {
    api.rustLoggerInit();
    console.info(path.join(__dirname, 'data_dir'));

    let re = await api.proxyCheckSocks5('45.145.57.215', 10519, 'nfxLjc', 'HpmpeN');
    console.info(re);
})();