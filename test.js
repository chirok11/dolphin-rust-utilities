const api = require('./');
const path = require('path');

(async() => {
    api.rustLoggerInit();
    console.info(path.join(__dirname, 'data_dir'));

    let re = await api.proxyCheckHttp('209.14.2.75', 9654, '11040', 'a93099efeab8');
    console.debug(re.replaceAll("\n", ''));
    console.info(JSON.parse(re));
})();