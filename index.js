const { existsSync, readFileSync } = require('fs')
const { join } = require('path')

const { platform, arch } = process

let nativeBinding = null
let localFileExisted = false
let loadError = null

function isMusl() {
  // For Node 10
  if (!process.report || typeof process.report.getReport !== 'function') {
    try {
      return readFileSync('/usr/bin/ldd', 'utf8').includes('musl')
    } catch (e) {
      return true
    }
  } else {
    const { glibcVersionRuntime } = process.report.getReport().header
    return !glibcVersionRuntime
  }
}

switch (platform) {
  case 'win32':
    switch (arch) {
      case 'x64':
        localFileExisted = existsSync(
          join(__dirname, 'dolphin-utilities-rust.win32-x64-msvc.node')
        )
        try {
          if (localFileExisted) {
            nativeBinding = require('./dolphin-utilities-rust.win32-x64-msvc.node')
          } else {
            nativeBinding = require('dolphin-utilities-rust-win32-x64-msvc')
          }
        } catch (e) {
          loadError = e
        }
        break
      case 'ia32':
        localFileExisted = existsSync(
          join(__dirname, 'dolphin-utilities-rust.win32-ia32-msvc.node')
        )
        try {
          if (localFileExisted) {
            nativeBinding = require('./dolphin-utilities-rust.win32-ia32-msvc.node')
          } else {
            nativeBinding = require('dolphin-utilities-rust-win32-ia32-msvc')
          }
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on Windows: ${arch}`)
    }
    break
  case 'darwin':
    switch (arch) {
      case 'x64':
        localFileExisted = existsSync(join(__dirname, 'dolphin-utilities-rust.darwin-x64.node'))
        try {
          if (localFileExisted) {
            nativeBinding = require('./dolphin-utilities-rust.darwin-x64.node')
          } else {
            nativeBinding = require('dolphin-utilities-rust-darwin-x64')
          }
        } catch (e) {
          loadError = e
        }
        break
      case 'arm64':
        localFileExisted = existsSync(
          join(__dirname, 'dolphin-utilities-rust.darwin-arm64.node')
        )
        try {
          if (localFileExisted) {
            nativeBinding = require('./dolphin-utilities-rust.darwin-arm64.node')
          } else {
            nativeBinding = require('dolphin-utilities-rust-darwin-arm64')
          }
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on macOS: ${arch}`)
    }
    break
  case 'linux':
    switch (arch) {
      case 'x64':
        localFileExisted = existsSync(
          join(__dirname, 'dolphin-utilities-rust.linux-x64-gnu.node')
        )
        try {
          if (localFileExisted) {
            nativeBinding = require('./dolphin-utilities-rust.linux-x64-gnu.node')
          } else {
            nativeBinding = require('dolphin-utilities-rust-linux-x64-gnu')
          }
        } catch (e) {
          loadError = e
        }
        break
      default:
        throw new Error(`Unsupported architecture on Linux: ${arch}`)
    }
    break
  default:
    throw new Error(`Unsupported OS: ${platform}, architecture: ${arch}`)
}

if (!nativeBinding) {
  if (loadError) {
    throw loadError
  }
  throw new Error(`Failed to load native binding`)
}

const { proxyCheckHttp, proxyCheckSocks5, loggerInit } = nativeBinding

module.exports.proxyCheckHttp = proxyCheckHttp
module.exports.proxyCheckSocks5 = proxyCheckSocks5
module.exports.loggerInit = loggerInit
