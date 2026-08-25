# A4N iOS WebView 壳

这是一个可在 Linux 上用 Theos 构建的最小 iOS `WKWebView` 壳。它不携带
A4N Server、数据库或 Sidecar；首次启动会连接当前部署的服务器：

```text
https://api.imggb.top/a4n/
```

这里填写 Web 入口，不要填写以 `/api` 结尾的 API 地址。也可以在应用右上角
“服务器”按钮中切换到其他地址。

地址会保存在 iOS 本地，之后直接打开。公网地址必须使用 HTTPS；局域网调试时，
HTTP 仅允许 localhost、`.local` 和私网 IP。

## 构建

需要 Linux Theos、iPhoneOS SDK 和 Clang。指定 Theos 目录后构建：

```bash
cd ios
THEOS=/path/to/theos make -B package FINALPACKAGE=1 PACKAGE_FORMAT=ipa
```

产物位于 `ios/packages/A4NRemote_0.1.0.ipa`。这是未使用 Apple 开发者证书签名的
IPA，安装工具需要在侧载时重签。
