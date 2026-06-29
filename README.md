# Pairee — Official Website

This repository and branch (`gh-pages`) contain the static source files for the official website of **Pairee**, a modern, dual-pane, modular terminal file manager (TUI) written in Rust.

The website is available at: [pairee.fitty.ar](https://pairee.fitty.ar) (or the corresponding GitHub Pages URL).

## 🚀 Branch Contents

This branch is exclusively dedicated to the web presentation of the project and contains:
- `index.html`: The main structure and content of the landing page.
- `style.css`: The custom stylesheet featuring a modern dark theme, gradients, and a fully responsive layout.
- `assets/`: Screenshots and graphics used across the website (including the "Wizard" mascot assets).
- `LICENSE`: The license file for this branch (MIT).

## 💻 Local Development

To view the website locally:
1. Clone this branch or ensure you are checked out to it:
   ```bash
   git checkout gh-pages
   ```
2. Open the `index.html` file in your preferred web browser, or spin up a simple development server:
   ```bash
   # Using Python
   python3 -m http.server 8000
   
   # Or using Node.js (npx)
   npx serve .
   ```

## 🛠️ Pairee Source Code

If you are looking for the source code of the **Pairee** file manager, along with compilation, installation, and contribution guidelines, please head over to the main repository branch:

🔗 **[Pairee Main Repository (master branch)](https://github.com/FittyAr/Pairee/tree/master)**

## 📄 License

Unlike the core Pairee application (which is licensed under GPL v3), the presentation files of this website and this branch are licensed under the **MIT License**. See the [LICENSE](LICENSE) file for details.
