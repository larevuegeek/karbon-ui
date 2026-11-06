// Vérifie la grammaire TextMate de la syntaxe HTML des templates `.ui`.
// Usage : <node> tools/vscode-karbon-ui/tests/highlighting.cjs <VS Code resources/app> [rust.tmLanguage.json]
// Avec une installation récente de VS Code, vscode-textmate est dans node_modules.asar :
// lancer alors le script avec le Node d'Electron (ELECTRON_RUN_AS_NODE=1 Code.exe …).
const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const extensionRoot = path.resolve(__dirname, '..');
const projectRoot = path.resolve(extensionRoot, '../..');
const app = process.argv[2];
if (!app) throw new Error('Pass the VS Code resources/app directory as first argument.');
function dependency(name) {
    for (const directory of ['node_modules', 'node_modules.asar']) {
        try { return require(path.join(app, directory, name)); }
        catch (error) { if (error.code !== 'MODULE_NOT_FOUND') throw error; }
    }
    throw new Error('Cannot load ' + name + ' from VS Code; use Electron Node for asar modules.');
}
const textmate = dependency('vscode-textmate');
const oniguruma = dependency('vscode-oniguruma');
const rustPath = process.argv[3] || path.join(app, 'extensions/rust/syntaxes/rust.tmLanguage.json');
const manifest = JSON.parse(fs.readFileSync(path.join(extensionRoot, 'package.json'), 'utf8'));
const config = JSON.parse(fs.readFileSync(path.join(extensionRoot, 'language-configuration.json'), 'utf8'));
const grammarPath = path.join(extensionRoot, manifest.contributes.grammars[0].path);
const snippets = JSON.parse(fs.readFileSync(path.join(extensionRoot, 'snippets/karbon-ui.json'), 'utf8'));

async function main() {
    const wasm = fs.readFileSync(path.join(app, 'node_modules.asar.unpacked/vscode-oniguruma/release/onig.wasm'));
    await oniguruma.loadWASM(wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength));
    const registry = new textmate.Registry({
        onigLib: Promise.resolve({
            createOnigScanner: patterns => new oniguruma.OnigScanner(patterns),
            createOnigString: value => new oniguruma.OnigString(value),
        }),
        loadGrammar: async scope => {
            const file = scope === 'text.karbon-ui' ? grammarPath : scope === 'source.rust' ? rustPath : null;
            if (!file) return null;
            return textmate.parseRawGrammar(fs.readFileSync(file, 'utf8'), file);
        },
    });
    const grammar = await registry.loadGrammar('text.karbon-ui');
    const baseline = grammar.tokenizeLine('', textmate.INITIAL).ruleStack;
    function tokenize(source, label) {
        let state = baseline;
        const lines = source.split(/\r?\n/);
        const result = lines.map(line => {
            const tokens = grammar.tokenizeLine(line, state);
            state = tokens.ruleStack;
            return { line, tokens: tokens.tokens };
        });
        assert(state.equals(baseline), 'Unclosed grammar state after ' + (label || ':\n' + source));
        return result;
    }
    function token(lines, line, text, occurrence = 0) {
        const record = lines[line];
        let offset = -1;
        for (let i = 0; i <= occurrence; i++) offset = record.line.indexOf(text, offset + 1);
        assert(offset >= 0, 'Missing test marker ' + text);
        return record.tokens.find(t => t.startIndex <= offset && t.endIndex > offset);
    }
    function has(lines, line, text, scope, occurrence = 0) {
        const found = token(lines, line, text, occurrence);
        assert(found && found.scopes.some(s => s.includes(scope)),
            text + ' (line ' + line + ') expected ' + scope + ', received ' + JSON.stringify(found));
    }
    function lacks(lines, line, text, scope, occurrence = 0) {
        const found = token(lines, line, text, occurrence);
        assert(found && !found.scopes.some(s => s.includes(scope)),
            text + ' (line ' + line + ') unexpected ' + scope + ': ' + JSON.stringify(found));
    }
    const source = [
        /* 0 */ '<!-- <div> {#if} not markup -->',
        /* 1 */ '<main class="page" data-exit="fade-out 200ms" aria-label=\'Main\'>',
        /* 2 */ '  <h1>Templates &amp; HTML &#233; {self.title}</h1>',
        /* 3 */ '  <input type="text" value={if self.ready { "}" } else { "x" }} oninput={Msg::Name} disabled>',
        /* 4 */ '  <Card title="a > } <" width=104 align=center>',
        /* 5 */ '    <widgets::Panel style:painter={Frame {',
        /* 6 */ '        image: None, radius: 32.*cx.scale,',
        /* 7 */ '    }} class:active={self.page == 1} />',
        /* 8 */ '  </Card>',
        /* 9 */ '  {#if self.items.len() > 0}',
        /* 10 */ '    {#each self.items as (i, item) (item.id)}',
        /* 11 */ '      <li onclick={Msg::Open(item.id)}>{item.name} : n° {i}</li>',
        /* 12 */ '    {:else}',
        /* 13 */ '      <p>Rien</p>',
        /* 14 */ '    {/each}',
        /* 15 */ '  {:else if self.loading}',
        /* 16 */ '    <k-split axis="horizontal"><slot name="left" /><slot /></k-split>',
        /* 17 */ '  {:else}',
        /* 18 */ '    {@include "form.ui" slot="body"}',
        /* 19 */ '  {/if}',
        /* 20 */ '  {@element volume(app, &cx)}',
        /* 21 */ '  <input type="range" oninput={|v| Msg::Set(3, v)} style="width:120px; gap:8px">',
        /* 22 */ '  <p>a < b, http://example.com // pas un commentaire (texte) ; fin</p>',
        /* 23 */ '  <span title={/* } /* nested } */ */ "x"}>{r#"} <p> {"#}</span>',
        /* 24 */ '  <k-virtual-list items={self.rows} row-height="24"></k-virtual-list>',
        /* 25 */ '</main>',
    ].join('\n');
    const t = tokenize(source);
    has(t, 0, 'not markup', 'comment.block');
    lacks(t, 0, 'div', 'entity.name.tag');
    has(t, 1, 'main', 'entity.name.tag.html');
    has(t, 1, 'class', 'entity.other.attribute-name');
    has(t, 1, 'page', 'string.quoted.double');
    has(t, 1, 'data', 'entity.other.attribute-name.data');
    has(t, 1, 'exit', 'entity.other.attribute-name.data');
    has(t, 1, 'aria-label', 'entity.other.attribute-name.aria');
    has(t, 1, 'Main', 'string.quoted.single');
    has(t, 2, 'Templates', 'text.karbon-ui');
    lacks(t, 2, 'Templates', 'string');
    has(t, 2, '&amp;', 'constant.character.entity');
    has(t, 2, '&#233;', 'constant.character.entity');
    has(t, 2, 'self', 'source.rust');
    has(t, 2, 'self', 'meta.embedded.block.rust');
    has(t, 3, 'if self', 'source.rust');
    has(t, 3, 'oninput', 'entity.name.function.event');
    has(t, 3, 'Name', 'source.rust');
    has(t, 3, 'disabled', 'entity.other.attribute-name');
    has(t, 4, 'Card', 'support.class.component');
    has(t, 4, '"a >', 'string.quoted');
    has(t, 4, '104', 'constant.numeric');
    has(t, 4, 'center', 'string.unquoted');
    has(t, 5, 'widgets', 'entity.name.namespace');
    has(t, 5, 'Panel', 'support.class.component');
    has(t, 5, 'style', 'keyword.control.directive');
    has(t, 5, 'painter', 'support.type.property-name');
    has(t, 5, 'Frame', 'source.rust');
    has(t, 6, 'radius', 'source.rust');
    has(t, 7, 'class', 'keyword.control.directive');
    has(t, 7, 'active', 'entity.other.attribute-name.class');
    has(t, 7, 'self.page', 'source.rust');
    has(t, 8, 'Card', 'support.class.component');
    has(t, 9, '#if', 'keyword.control.conditional');
    has(t, 9, 'len', 'source.rust');
    has(t, 9, 'len', 'meta.embedded.block.rust');
    has(t, 10, '#each', 'keyword.control.loop');
    has(t, 10, 'self.items', 'source.rust');
    has(t, 10, 'as', 'keyword.control.loop.as');
    has(t, 10, 'item.id', 'source.rust');
    has(t, 11, 'li', 'entity.name.tag.html');
    has(t, 11, 'onclick', 'entity.other.attribute-name.event');
    has(t, 11, 'item.name', 'source.rust');
    lacks(t, 11, ' : n', 'source.rust');
    has(t, 12, ':else', 'keyword.control.conditional');
    has(t, 14, '/each', 'keyword.control');
    has(t, 15, ':else if', 'keyword.control.conditional');
    has(t, 15, 'loading', 'source.rust');
    has(t, 16, 'k-split', 'entity.name.tag.custom');
    has(t, 16, 'axis', 'entity.other.attribute-name');
    has(t, 16, 'slot', 'entity.name.tag.slot');
    has(t, 16, 'k-split', 'entity.name.tag.custom', 1);
    has(t, 18, '@include', 'keyword.control.import');
    has(t, 18, 'form.ui', 'string.quoted.double');
    has(t, 18, 'slot', 'entity.other.attribute-name');
    has(t, 19, '/if', 'keyword.control');
    has(t, 20, '@element', 'keyword.control.element');
    has(t, 20, 'volume', 'source.rust');
    has(t, 21, 'Set', 'source.rust');
    has(t, 21, 'width:120px', 'string.quoted');
    has(t, 21, 'width:120px', 'support.type.property-name.css');
    has(t, 21, '120', 'constant.numeric.css');
    has(t, 21, '>', 'punctuation.definition.tag.end', 0);
    has(t, 22, 'b, http', 'text.karbon-ui');
    lacks(t, 22, '// pas', 'comment');
    lacks(t, 22, '(texte)', 'source.rust');
    lacks(t, 22, 'b, http', 'entity.name.tag');
    has(t, 22, 'p>', 'entity.name.tag.html', 1);
    has(t, 23, 'nested', 'comment');
    has(t, 23, '<p>', 'string');
    has(t, 23, 'span', 'entity.name.tag.html', 1);
    has(t, 24, 'k-virtual-list', 'entity.name.tag.custom');
    has(t, 24, 'row-height', 'entity.other.attribute-name');
    has(t, 25, 'main', 'entity.name.tag.html');
    for (const line of t) {
        assert(!line.tokens.some(tk => tk.scopes.some(s => s.startsWith('invalid'))), 'Invalid token on: ' + line.line);
    }

    const ruleSources = [...(config.onEnterRules || []).flatMap(r => [r.beforeText, r.afterText].filter(Boolean)),
        ...Object.values(config.indentationRules), config.wordPattern];
    const rx = ruleSources.map(source => new RegExp(source));
    const { increaseIndentPattern: inc, decreaseIndentPattern: dec } = config.indentationRules;
    const increase = new RegExp(inc), decrease = new RegExp(dec);
    for (const line of ['<div class="a">', '<Card title="x">', '  {#if self.a > 1}', '{#each xs as x (x.id)}', '{:else}',
        '{:else if b}', '<k-split axis="horizontal">', '<widgets::Panel>', '<dialog onclose={Msg::Close}>', '<!--',
        '<div style:painter={Frame {'])
        assert(increase.test(line), 'Should indent after: ' + line);
    for (const line of ['<div></div>', '<input type="text">', '<img src={x} alt="y">', '<hr>', '<slot />', '<br>',
        '{@include "a.ui"}', '<p>Bonjour</p>', '{/if}', '<Card title="x" />'])
        assert(!increase.test(line), 'Should not indent after: ' + line);
    for (const line of ['</div>', '  {/if}', '{/each}', '{:else}', '{:else if x}', '-->', '}}>'])
        assert(decrease.test(line), 'Should outdent: ' + line);
    for (const line of ['<div>', '{#if x}', 'texte'])
        assert(!decrease.test(line), 'Should not outdent: ' + line);
    assert(rx.length > 0);

    assert(Object.keys(snippets).length >= 25, 'Expected the HTML snippet set');
    let snippetCount = 0;
    for (const [name, snippet] of Object.entries(snippets)) {
        // Développe les champs : $1 / ${1} → "v", ${1:défaut} → défaut, ${1|a,b|} → a.
        let body = Array.isArray(snippet.body) ? snippet.body.join('\n') : snippet.body;
        body = body.replace(/\$(?:\d+|\{\d+\})/g, 'v');
        for (let previous = ''; previous !== body;) {
            previous = body;
            body = body.replace(/\$\{\d+:([^{}$]*)\}/g, '$1').replace(/\$\{\d+\|([^,|]*)[^|]*\|\}/g, '$1');
        }
        assert(snippet.prefix && snippet.description, 'Snippet metadata: ' + name);
        assert(!/\$\{/.test(body), 'Unexpanded placeholder in snippet ' + name + ': ' + body);
        assert(!/on_click|on_change|<Text>|<Column|<Row|<If |<For |<include /.test(body), 'Old syntax in snippet ' + name);
        tokenize(body, 'snippet ' + name);
        snippetCount++;
    }

    let count = 0;
    function walk(directory) {
        for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
            const file = path.join(directory, entry.name);
            if (entry.isDirectory()) walk(file);
            else if (entry.name.endsWith('.ui')) {
                const lines = tokenize(fs.readFileSync(file, 'utf8'), file);
                for (const line of lines)
                    assert(!line.tokens.some(tk => tk.scopes.some(s => s.startsWith('invalid'))), 'Invalid token in ' + file + ': ' + line.line);
                count++;
            }
        }
    }
    walk(path.join(projectRoot, 'examples'));
    walk(path.join(projectRoot, 'tests'));
    console.log('Grammar OK: HTML tags, attributes, entities, comments, Svelte blocks, Rust embedding, '
        + snippetCount + ' snippets and ' + count + ' project templates.');
    registry.dispose();
}
main().catch(error => { console.error(error); process.exitCode = 1; });
