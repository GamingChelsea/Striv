const fs = require("fs");
const os = require("os");
const path = require("path");
const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

let client;

function findServerExe() {
  const candidates = [];
  if (vscode.workspace.workspaceFolders) {
    for (const folder of vscode.workspace.workspaceFolders) {
      candidates.push(path.join(folder.uri.fsPath, "target", "release", "striv.exe"));
      candidates.push(path.join(folder.uri.fsPath, "target", "release", "striv-cli.exe"));
      candidates.push(path.join(folder.uri.fsPath, "target", "debug", "striv-cli.exe"));
    }
  }
  candidates.push(path.join(__dirname, "striv.exe"));
  candidates.push(path.join(__dirname, "..", "target", "release", "striv.exe"));
  candidates.push(path.join(__dirname, "..", "target", "release", "striv-cli.exe"));

  for (const c of candidates) {
    if (fs.existsSync(c)) {
      return c;
    }
  }
  return "striv";
}

function activate(_context) {
  const serverExe = findServerExe();
  const serverOptions = { command: serverExe, args: ["lsp"] };
  const clientOptions = { documentSelector: [{ scheme: "file", language: "striv" }] };

  client = new LanguageClient("strivLsp", "Striv Language Server", serverOptions, clientOptions);
  client.start();
}

function deactivate() {
  return client ? client.stop() : undefined;
}

module.exports = { activate, deactivate };
