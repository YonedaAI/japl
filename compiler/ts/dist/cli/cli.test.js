import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { parseConfig } from './config.js';
import { buildToWat } from './build.js';
import * as fs from 'node:fs';
import * as path from 'node:path';
import * as os from 'node:os';
// ─── Config Parser Tests ───
describe('Config parser', () => {
    it('parses a basic japl.toml', () => {
        const config = parseConfig(`[package]
name = "myapp"
version = "0.1.0"
entry = "src/main.japl"

[dependencies]

[dev-dependencies]
`);
        expect(config.package?.name).toBe('myapp');
        expect(config.package?.version).toBe('0.1.0');
        expect(config.package?.entry).toBe('src/main.japl');
    });
    it('handles empty sections', () => {
        const config = parseConfig(`[package]
name = "test"

[dependencies]

[dev-dependencies]
`);
        expect(config.dependencies).toEqual({});
        expect(config['dev-dependencies']).toEqual({});
    });
    it('skips comments and blank lines', () => {
        const config = parseConfig(`# This is a comment
[package]
# Another comment
name = "test"

version = "1.0.0"
`);
        expect(config.package?.name).toBe('test');
        expect(config.package?.version).toBe('1.0.0');
    });
    it('handles single-quoted values', () => {
        const config = parseConfig(`[package]
name = 'single-quoted'
`);
        expect(config.package?.name).toBe('single-quoted');
    });
    it('handles unquoted values', () => {
        const config = parseConfig(`[package]
version = 0.1.0
`);
        expect(config.package?.version).toBe('0.1.0');
    });
    it('handles multiple sections', () => {
        const config = parseConfig(`[package]
name = "app"

[dependencies]
http = "0.1.0"
json = "1.2.3"

[dev-dependencies]
test-lib = "0.5.0"
`);
        expect(config.package?.name).toBe('app');
        expect(config.dependencies?.http).toBe('0.1.0');
        expect(config.dependencies?.json).toBe('1.2.3');
        expect(config['dev-dependencies']?.['test-lib']).toBe('0.5.0');
    });
    it('returns empty config for empty input', () => {
        const config = parseConfig('');
        expect(config).toEqual({});
    });
});
// ─── Build Tests (WASM target) ───
describe('Build pipeline (WASM)', () => {
    it('compiles a simple function to WAT stub', () => {
        const wat = buildToWat(`fn add(x: Int, y: Int) -> Int { x + y }`);
        expect(wat).toContain('(module');
    });
    it('throws on invalid syntax', () => {
        expect(() => buildToWat('fn {')).toThrow();
    });
});
// ─── japl new Tests ───
describe('japl new scaffolding', () => {
    let tmpDir;
    beforeEach(() => {
        tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'japl-new-test-'));
    });
    afterEach(() => {
        fs.rmSync(tmpDir, { recursive: true, force: true });
    });
    it('creates project directory structure', () => {
        const projectDir = path.join(tmpDir, 'testproj');
        fs.mkdirSync(path.join(projectDir, 'src'), { recursive: true });
        fs.writeFileSync(path.join(projectDir, 'japl.toml'), `[package]
name = "testproj"
version = "0.1.0"
entry = "src/main.japl"

[dependencies]

[dev-dependencies]
`);
        fs.writeFileSync(path.join(projectDir, 'src', 'main.japl'), `fn main() {
  println("Hello from testproj!")
}
`);
        fs.writeFileSync(path.join(projectDir, '.gitignore'), `build/
.japl-build/
*.wasm
*.wat
node_modules/
`);
        // Verify structure
        expect(fs.existsSync(path.join(projectDir, 'japl.toml'))).toBe(true);
        expect(fs.existsSync(path.join(projectDir, 'src', 'main.japl'))).toBe(true);
        expect(fs.existsSync(path.join(projectDir, '.gitignore'))).toBe(true);
    });
    it('generates valid japl.toml', () => {
        const projectDir = path.join(tmpDir, 'myapp');
        fs.mkdirSync(path.join(projectDir, 'src'), { recursive: true });
        const toml = `[package]
name = "myapp"
version = "0.1.0"
entry = "src/main.japl"

[dependencies]

[dev-dependencies]
`;
        fs.writeFileSync(path.join(projectDir, 'japl.toml'), toml);
        const config = parseConfig(fs.readFileSync(path.join(projectDir, 'japl.toml'), 'utf-8'));
        expect(config.package?.name).toBe('myapp');
        expect(config.package?.version).toBe('0.1.0');
        expect(config.package?.entry).toBe('src/main.japl');
    });
    it('build produces WAT output', () => {
        const source = `fn hello() -> String { "world" }`;
        const wat = buildToWat(source);
        expect(wat).toContain('(module');
    });
});
//# sourceMappingURL=cli.test.js.map