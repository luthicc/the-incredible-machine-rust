"""Inline the built web player into one HTML file that opens from file://.

Browsers refuse to load module scripts and .wasm over file://, so the
wasm-bindgen glue, its snippet, the wasm binary and the icon are embedded.
Usage: python3 src/web/bundle.py web OUTPUT.html
"""

import base64
import re
import sys
from pathlib import Path

web, out = Path(sys.argv[1]), Path(sys.argv[2])
pkg = web / "pkg"

glue = (pkg / "tim_bevy.js").read_text()
for snippet in sorted(pkg.glob("snippets/*/*.js")):
    rel = snippet.relative_to(pkg).as_posix()
    pattern = rf"^import \{{[^}}]*\}} from '\./{re.escape(rel)}';$"
    glue, n = re.subn(pattern, lambda _: snippet.read_text(), glue, flags=re.M)
    if n != 1:
        sys.exit(f"snippet import not found: {rel}")
glue = re.sub(r"^export \{[^}]*\};?$", "", glue, flags=re.M)
glue = re.sub(r"^export (function|async function|class|const|let) ", r"\1 ", glue, flags=re.M)
if re.search(r"^\s*(import|export)\b", glue, flags=re.M):
    sys.exit("unhandled import/export left in wasm-bindgen glue")

wasm = base64.b64encode((pkg / "tim_bevy_bg.wasm").read_bytes()).decode()
icon = "data:image/svg+xml;base64," + base64.b64encode((web / "icon.svg").read_bytes()).decode()

loader = f"""
{glue}
function wasmBytes() {{
  const text = document.getElementById('wasm').textContent.trim();
  if (Uint8Array.fromBase64) return Uint8Array.fromBase64(text);
  const bin = atob(text), bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}}
const init = () => __wbg_init({{ module_or_path: wasmBytes() }});
"""

html = (web / "index.html").read_text()
html = html.replace("./icon.svg", icon)
import_line = "import init, {start_game,audio_samples,go_back} from './pkg/tim_bevy.js';"
if import_line not in html:
    sys.exit("index.html import line changed; update bundle.py")
html = html.replace(import_line, loader, 1)
html = html.replace(
    '<script type="module">',
    f'<script type="application/octet-stream" id="wasm">{wasm}</script>\n<script type="module">',
    1,
)
out.write_text(html)
print(f"wrote {out} ({out.stat().st_size / 1e6:.1f} MB)")
