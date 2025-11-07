# BitQuan Documentation

Live documentation site powered by Docsify.

## 📚 Browse Documentation

**Live Site**: [https://alphab135.github.io/BitQuan/](https://alphab135.github.io/BitQuan/)

Or browse directly in GitHub:
- [📖 Documentation Index](INDEX.md)
- [🚀 Getting Started](getting-started/)
- [⚙️ CLI Reference](cli/)
- [🛠️ Development](dev/)
- [🖥️ Operations](ops/)
- [🔒 Security](security/)

## 🌐 Local Development

To view documentation locally:

```bash
# Option 1: Python HTTP server
cd docs
python3 -m http.server 3000
# Visit http://localhost:3000

# Option 2: Docsify CLI (optional)
npm i -g docsify-cli
docsify serve docs
# Visit http://localhost:3000
```

## 🔧 Documentation Structure

```
docs/
├── index.html           # Docsify entry point
├── INDEX.md             # Documentation home
├── _sidebar.md          # Navigation sidebar
├── _404.md              # 404 page
├── .nojekyll            # GitHub Pages config
├── getting-started/     # Installation & quick start
├── concepts/            # Core concepts
├── cli/                 # Command-line reference
├── dev/                 # Development guides
├── ops/                 # Operations & deployment
├── testnet/             # Testnet documentation
├── security/            # Security policies & audits
├── releases/            # Release notes
├── guides/              # How-to guides
├── wallet/              # Wallet documentation
├── rpc/                 # RPC API reference
└── spec/                # Technical specifications
```

## ✨ Features

- 🔍 **Full-text search** - Find anything instantly
- 📱 **Mobile-friendly** - Responsive design
- 🎨 **Syntax highlighting** - Code blocks for Rust, Bash, TOML, JSON
- 🔗 **Deep linking** - Share links to specific sections
- 📄 **Pagination** - Navigate between pages
- 📋 **Copy code** - One-click code copying
- 🖼️ **Image zoom** - Click to enlarge images
- 🌙 **Clean UI** - Easy to read and navigate

## 📝 Contributing

Documentation improvements are welcome! See [CONTRIBUTING.md](../CONTRIBUTING.md).

### Adding New Pages

1. Create markdown file in appropriate directory
2. Add to `_sidebar.md` navigation
3. Follow documentation standards (H1, updated date, etc.)
4. Test locally before committing

### Documentation Standards

- One H1 heading per file
- Include "Last Updated: YYYY-MM-DD" at top
- Use relative links (e.g., `../ops/RUNBOOK.md`)
- Add code language tags for syntax highlighting
- Keep lines under 100 characters where practical

## 🚀 GitHub Pages Deployment

GitHub Pages automatically serves `docs/` directory:

1. Repository Settings → Pages
2. Source: Deploy from branch `main`
3. Folder: `/docs`
4. Save

Site will be available at: `https://alphab135.github.io/BitQuan/`

## 📊 Analytics

Run analysis tools:

```bash
# Check for broken links and duplicates
python3 tools/analyze_docs.py

# Fix broken links automatically
python3 tools/smart_fix_links.py
```

---

**Questions?** Open an issue on [GitHub](https://github.com/AlphaB135/BitQuan/issues).
