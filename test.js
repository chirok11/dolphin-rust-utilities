const api = require('./');
const path = require('path');

(async() => {
    api.loggerInit();
    let res = api.archivateFolder('archive.tar', __dirname, ['package.json', 'npm/*/package.json']);
    console.info(res);
    //let res = await api.proxyCheckHttp('185.149.40.163', 11361, 'DolphinSupportOnly', 'ges5oSXdsE6i7CWt');
    //let res = await api.proxyCheckSocks5('194.183.168.4', 14521, 'user3021', 'keeT8xei');
    //let res = await api.proxyCheckHttp('77.77.151.71', 34567, 'dmitri', '1234dmitri5678')
    //console.info(res);
    //let res = await api.proxyCheckHttp('proxy.packetstream.io', 31112, 'chirok11', 'HHBABE7ZJtikFBpN_country-Finland_session-0bDeQNkT');
    //console.info(res);

    //res = await api.proxyCheckHttp('proxy.packetstream.io', 31112, 'chirok11', 'HHBABE7ZJtikFBpN_country-Luxembourg_session-wqfaAeG0');
    //console.info(res);

    //res = await api.proxyCheckHttp('proxy.packetstream.io', 31112, 'chirok11', 'HHBABE7ZJtikFBpN_country-Luxembourg_session-ln6QSXQq');
    //console.info(res);

    //res = await api.proxyCheckHttp('proxy.packetstream.io', 31112, 'chirok11', 'HHBABE7ZJtikFBpN_country-Germany_session-GAeb1WwN');
    //console.info(res);
})();
// socks5://109.248.7.222:10964:in9536:9f0eab
// socks5://u3325:carazz@109.195.35.233:4425
// socks5://80.66.72.63:8000:QPhT4k:mDx1jd
// socks5://194.183.168.4:14521:user3021:keeT8xei
// socks5://velikrus184331:7299056@94.142.140.109:13890