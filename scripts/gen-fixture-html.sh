#!/bin/bash
# Generate HTML pages showing test fixtures for each language
#
# Creates one HTML page per language in the docs/fixtures directory,
# with a table showing YAY input alongside the language-specific representation.

set -eo pipefail

HERE=$(dirname "${BASH_SOURCE[0]}")
ROOT="$HERE/.."
TEST_DIR="$ROOT/test"
OUTPUT_DIR="$ROOT/docs/fixtures"

# Get language extension
get_ext() {
	case "$1" in
	c) echo "c" ;;
	go) echo "go" ;;
	java) echo "java" ;;
	js) echo "js" ;;
	python) echo "py" ;;
	rust) echo "rs" ;;
	scheme) echo "scm" ;;
	esac
}

# Get language display name
get_name() {
	case "$1" in
	c) echo "C" ;;
	go) echo "Go" ;;
	java) echo "Java" ;;
	js) echo "JavaScript" ;;
	python) echo "Python" ;;
	rust) echo "Rust" ;;
	scheme) echo "Scheme" ;;
	esac
}

# Get syntax highlighting class
get_highlight() {
	case "$1" in
	c) echo "c" ;;
	go) echo "go" ;;
	java) echo "java" ;;
	js) echo "javascript" ;;
	python) echo "python" ;;
	rust) echo "rust" ;;
	scheme) echo "scheme" ;;
	esac
}

# HTML escape function
html_escape() {
	local text="$1"
	text="${text//&/&amp;}"
	text="${text//</&lt;}"
	text="${text//>/&gt;}"
	text="${text//\"/&quot;}"
	printf '%s' "$text"
}

# Generate HTML header
gen_header() {
	local lang="$1"
	local name
	name=$(get_name "$lang")

	cat <<'EOF'
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
EOF
	echo "  <title>YAY Test Fixtures - ${name}</title>"
	cat <<'EOF'
  <link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/github.min.css">
  <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
  <script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/scheme.min.js"></script>
  <script>hljs.highlightAll();</script>
  <style>
    :root {
      --bg-color: #ffffff;
      --text-color: #24292e;
      --border-color: #e1e4e8;
      --header-bg: #f6f8fa;
      --code-bg: #f6f8fa;
      --link-color: #0366d6;
      --hover-bg: #f3f4f6;
    }
    
    @media (prefers-color-scheme: dark) {
      :root {
        --bg-color: #0d1117;
        --text-color: #c9d1d9;
        --border-color: #30363d;
        --header-bg: #161b22;
        --code-bg: #161b22;
        --link-color: #58a6ff;
        --hover-bg: #21262d;
      }
    }
    
    * {
      box-sizing: border-box;
    }
    
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
      background-color: var(--bg-color);
      color: var(--text-color);
      line-height: 1.6;
      margin: 0;
      padding: 20px;
    }
    
    .container {
      max-width: 1400px;
      margin: 0 auto;
    }
    
    h1 {
      border-bottom: 1px solid var(--border-color);
      padding-bottom: 10px;
      margin-bottom: 20px;
    }
    
    .nav {
      margin-bottom: 20px;
      padding: 10px 0;
      border-bottom: 1px solid var(--border-color);
    }
    
    .nav a {
      color: var(--link-color);
      text-decoration: none;
      margin-right: 15px;
      padding: 5px 10px;
      border-radius: 4px;
    }
    
    .nav a:hover {
      background-color: var(--hover-bg);
    }
    
    .nav a.active {
      background-color: var(--link-color);
      color: white;
    }
    
    table {
      width: 100%;
      border-collapse: collapse;
      margin-top: 20px;
    }
    
    th, td {
      border: 1px solid var(--border-color);
      padding: 12px;
      text-align: left;
      vertical-align: top;
    }
    
    th {
      background-color: var(--header-bg);
      font-weight: 600;
      position: sticky;
      top: 0;
    }
    
    tr:hover {
      background-color: var(--hover-bg);
    }
    
    .fixture-name {
      font-weight: 500;
      white-space: nowrap;
    }
    
    pre {
      margin: 0;
      padding: 8px;
      background-color: var(--code-bg);
      border-radius: 4px;
      overflow-x: auto;
      font-size: 13px;
    }
    
    code {
      font-family: 'SFMono-Regular', Consolas, 'Liberation Mono', Menlo, monospace;
    }
    
    .yay-col {
      width: 40%;
    }
    
    .lang-col {
      width: 50%;
    }
    
    .name-col {
      width: 10%;
    }
    
    .stats {
      margin-top: 20px;
      padding: 10px;
      background-color: var(--header-bg);
      border-radius: 4px;
      font-size: 14px;
    }
  </style>
</head>
<body>
  <div class="container">
EOF
	echo "    <h1>YAY Test Fixtures - ${name}</h1>"
}

# Generate navigation
gen_nav() {
	local current="$1"
	local nav_lang nav_name

	echo '    <nav class="nav">'
	for nav_lang in c go java js python rust scheme; do
		nav_name=$(get_name "$nav_lang")
		if [[ "$nav_lang" == "$current" ]]; then
			echo "      <a href=\"${nav_lang}.html\" class=\"active\">${nav_name}</a>"
		else
			echo "      <a href=\"${nav_lang}.html\">${nav_name}</a>"
		fi
	done
	echo '    </nav>'
}

# Generate table header
gen_table_header() {
	local lang="$1"
	local name
	name=$(get_name "$lang")

	cat <<EOF
    <table>
      <thead>
        <tr>
          <th class="name-col">Fixture</th>
          <th class="yay-col">YAY Input</th>
          <th class="lang-col">${name} Representation</th>
        </tr>
      </thead>
      <tbody>
EOF
}

# Generate table row
gen_table_row() {
	local name="$1"
	local yay_content="$2"
	local lang_content="$3"
	local lang="$4"
	local highlight
	highlight=$(get_highlight "$lang")

	local escaped_yay escaped_lang
	escaped_yay=$(html_escape "$yay_content")
	escaped_lang=$(html_escape "$lang_content")

	cat <<EOF
        <tr>
          <td class="fixture-name">${name}</td>
          <td><pre><code class="language-yaml">${escaped_yay}</code></pre></td>
          <td><pre><code class="language-${highlight}">${escaped_lang}</code></pre></td>
        </tr>
EOF
}

# Generate HTML footer
gen_footer() {
	local count="$1"

	cat <<EOF
      </tbody>
    </table>
    <div class="stats">
      Total fixtures: ${count}
    </div>
  </div>
</body>
</html>
EOF
}

# Generate HTML page for a language
gen_lang_page() {
	local lang="$1"
	local ext
	ext=$(get_ext "$lang")
	local output_file="$OUTPUT_DIR/${lang}.html"
	local count=0

	echo "Generating ${lang}.html..."

	{
		gen_header "$lang"
		gen_nav "$lang"
		gen_table_header "$lang"

		# Get sorted list of fixtures
		for yay_file in $(find "$TEST_DIR/yay" -name "*.yay" -type f | sort); do
			local base
			base=$(basename "$yay_file" .yay)
			local lang_file="$TEST_DIR/${ext}/${base}.${ext}"

			if [[ -f "$lang_file" ]]; then
				local yay_content lang_content
				yay_content=$(cat "$yay_file")
				lang_content=$(cat "$lang_file")

				gen_table_row "$base" "$yay_content" "$lang_content" "$lang"
				((count++)) || true
			fi
		done

		gen_footer "$count"
	} >"$output_file"

	echo "  Generated $count fixtures"
}

# Generate index page
gen_index_page() {
	local output_file="$OUTPUT_DIR/index.html"

	echo "Generating index.html..."

	cat <<'EOF' >"$output_file"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>YAY Test Fixtures</title>
  <style>
    :root {
      --bg-color: #ffffff;
      --text-color: #24292e;
      --border-color: #e1e4e8;
      --header-bg: #f6f8fa;
      --link-color: #0366d6;
      --hover-bg: #f3f4f6;
    }
    
    @media (prefers-color-scheme: dark) {
      :root {
        --bg-color: #0d1117;
        --text-color: #c9d1d9;
        --border-color: #30363d;
        --header-bg: #161b22;
        --link-color: #58a6ff;
        --hover-bg: #21262d;
      }
    }
    
    body {
      font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
      background-color: var(--bg-color);
      color: var(--text-color);
      line-height: 1.6;
      margin: 0;
      padding: 20px;
    }
    
    .container {
      max-width: 800px;
      margin: 0 auto;
    }
    
    h1 {
      border-bottom: 1px solid var(--border-color);
      padding-bottom: 10px;
    }
    
    .lang-list {
      list-style: none;
      padding: 0;
    }
    
    .lang-list li {
      margin: 10px 0;
    }
    
    .lang-list a {
      display: block;
      padding: 15px 20px;
      background-color: var(--header-bg);
      border: 1px solid var(--border-color);
      border-radius: 6px;
      color: var(--link-color);
      text-decoration: none;
      font-size: 18px;
    }
    
    .lang-list a:hover {
      background-color: var(--hover-bg);
    }
    
    .description {
      margin-bottom: 30px;
      color: var(--text-color);
    }
  </style>
</head>
<body>
  <div class="container">
    <h1>YAY Test Fixtures</h1>
    <p class="description">
      These pages show the YAY test fixtures alongside their expected representations
      in each supported programming language. Select a language to view its fixtures.
    </p>
    <ul class="lang-list">
      <li><a href="c.html">C</a></li>
      <li><a href="go.html">Go</a></li>
      <li><a href="java.html">Java</a></li>
      <li><a href="js.html">JavaScript</a></li>
      <li><a href="python.html">Python</a></li>
      <li><a href="rust.html">Rust</a></li>
      <li><a href="scheme.html">Scheme</a></li>
    </ul>
  </div>
</body>
</html>
EOF
}

# Main
main() {
	# Create output directory
	mkdir -p "$OUTPUT_DIR"

	echo "Generating HTML fixture pages..."
	echo

	# Generate page for each language
	for lang in c go java js python rust scheme; do
		gen_lang_page "$lang"
	done

	# Generate index page
	gen_index_page

	echo
	echo "Done! Output written to $OUTPUT_DIR/"
}

main "$@"
