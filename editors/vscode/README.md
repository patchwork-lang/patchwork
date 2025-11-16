# Patchwork VS Code Extension

This repository includes a VS Code language extension for Patchwork (`.pw` files).

## Quick Install

To install the extension for this workspace:

```bash
# From the repository root
npm install -g @vscode/vsce
vsce package
code --install-extension patchwork-vscode-0.0.1.vsix
```

## Alternative: Development Mode

For active development, you can run the extension directly:

1. Open this folder in VS Code
2. Press `F5` to launch Extension Development Host
3. The new window will have syntax highlighting enabled

## Manual Installation (Recommended)

1. Install vsce if you don't have it:
   ```bash
   npm install -g @vscode/vsce
   ```

2. Package the extension:
   ```bash
   vsce package
   ```
   This creates `patchwork-vscode-0.0.1.vsix`

3. Install the extension:
   - In VS Code: `Cmd+Shift+P` → "Extensions: Install from VSIX..."
   - Select the `.vsix` file

4. Reload VS Code

## Verify Installation

1. Open any `.pw` file in this repository
2. Check the language indicator in the bottom-right (should say "Patchwork")
3. You should see syntax highlighting

## Files

- `package.json` - Extension manifest
- `syntaxes/patchwork.tmLanguage.json` - TextMate grammar
- `language-configuration.json` - Language configuration (brackets, comments, etc.)
- `.vscode/settings.json` - Workspace settings for file associations
- `.vscode/extensions.json` - Recommended extensions for this workspace

## Troubleshooting

If syntax highlighting isn't working:

1. Check the language mode: Click the language indicator in the bottom-right corner
2. Select "Patchwork" from the language picker
3. If "Patchwork" isn't listed, the extension isn't installed correctly

## Development

To modify the syntax highlighting:

1. Edit `syntaxes/patchwork.tmLanguage.json`
2. Reload VS Code (`Cmd+R` in Extension Development Host)
3. Test with `.pw` files in the `test/` and `examples/` directories
