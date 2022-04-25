const api = require('./');
const path = require('path');
const { EventEmitter } = require('events');

(async() => {
    api.rustLoggerInit();
    console.info(path.join(__dirname, 'data_dir'));

    // 209.14.2.75:6000:11060:2ae704094cf7
    // let re = await api.proxyCheckHttp('209.14.2.75', 6000, '11060', '2ae704094cf7');
    const emitter = new EventEmitter();
    emitter.on('progress', (data) => {
        console.info(data);
    });
    let re = new api.HttpFileDownloader(emitter.emit.bind(emitter));
    let result = await re.downloadFile('http://91.210.165.92:83/dolphin-anty/anty-browser/releases/download/v83/mac-arm.zip', 'hello-world.zip');
    console.info(result);
})();