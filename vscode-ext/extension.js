const fs = require("fs");
const os = require("os");
const path = require("path");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

function activate(_context) {
  const sourceExe = path.join(__dirname, "..", "target", "release", "striv-cli.exe");
  const shadowExe = path.join(os.tmpdir(), "striv-lsp.exe");

  try {
    fs.copyFileSync(sourceExe, shadowExe);
  } catch (_err) {
    // If shadowExe is already running, reuse it
  }

  const serverExe = fs.existsSync(shadowExe) ? shadowExe : sourceExe;
  const serverOptions = { command: serverExe, args: ["lsp"] };
  const clientOptions = { documentSelector: [{ scheme: "file", language: "striv" }] };

  client = new LanguageClient("strivLsp", "Striv Language Server", serverOptions, clientOptions);
  client.start();
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
