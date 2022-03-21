const api = require('./');
const path = require('path');

(async() => {
    api.rustLoggerInit();
    console.info(path.join(__dirname, 'data_dir'));

    // 209.14.2.75:6000:11060:2ae704094cf7
    let re = await api.proxyCheckHttp('209.14.2.75', 6000, '11060', '2ae704094cf7');
    console.debug(re.replaceAll("\n", ''));
    console.info(JSON.parse(re));
})();