const path = require("path");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

function activate(_context) {
  const serverExe = path.join(__dirname, "..", "target", "release", "striv-cli.exe");
  const serverOptions = { command: serverExe, args: ["lsp"] };
  const clientOptions = { documentSelector: [{ scheme: "file", language: "striv" }] };

  client = new LanguageClient("strivLsp", "Striv Language Server", serverOptions, clientOptions);
  client.start();
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
