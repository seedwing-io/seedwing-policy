## JavaScript example
This directory contains an example that used the Seedwing WebAssembly
Component Model module and types.

### Building
First we need to generate bindings for the webassembly interface types:
```console
$ npm run bindings

> js@1.0.0 bindings
> npx jco transpile $npm_package_config_wasm_file -o dist -w


Transpiled JS Component Files:

 - dist/exports/engine.d.ts                          4.97 KiB
 - dist/imports/environment.d.ts                     0.09 KiB
 - dist/imports/exit.d.ts                            0.16 KiB
 - dist/imports/filesystem.d.ts                      2.31 KiB
 - dist/imports/preopens.d.ts                        0.47 KiB
 - dist/imports/random.d.ts                           0.1 KiB
 - dist/imports/streams.d.ts                         0.39 KiB
 - dist/seedwing_policy-engine-component.core.wasm    167 MiB
 - dist/seedwing_policy-engine-component.core2.wasm  14.2 KiB
 - dist/seedwing_policy-engine-component.d.ts        0.47 KiB
 - dist/seedwing_policy-engine-component.js
```
These generated files will then be used by [index.mjs].

### Running
```console
$ npm run example

> js@1.0.0 example
> node index.mjs

Seedwing Policy Engine version: 0.1.0
EvaluationResult:  {
  input: { tag: 'string', val: '{"name":"goodboy","trained":true}' },
  ty: {
    name: { package: { path: [ 'wit', [length]: 1 ] }, name: 'dog' },
    metadata: {
      documentation: null,
      unstable: false,
      deprecation: null,
      reporting: { severity: 'none', explanation: null, authoritative: false }
    },
    examples: [ [length]: 0 ],
    parameters: [ [length]: 0 ],
    inner: {
      tag: 'object',
      val: {
        fields: [
          { name: 'name', optional: false },
          { name: 'trained', optional: false },
          [length]: 2
        ]
      }
    }
  },
  rationale: { tag: 'not-an-object' },
  output: 'Identity'
}
```

[index.mjs]: engine/js/index.mjs
