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
      candidates.push(path.join(folder.uri.fsPath, "target", "release", "striv-cli.exe"));
      candidates.push(path.join(folder.uri.fsPath, "target", "debug", "striv-cli.exe"));
    }
  }
  candidates.push(path.join(__dirname, "..", "target", "release", "striv-cli.exe"));
  candidates.push("R:\\Projekte\\Rust\\Striv\\target\\release\\striv-cli.exe");

  for (const c of candidates) {
    if (fs.existsSync(c)) {
      return c;
    }
  }
  return candidates[candidates.length - 1];
}

function activate(_context) {
  const sourceExe = findServerExe();
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
