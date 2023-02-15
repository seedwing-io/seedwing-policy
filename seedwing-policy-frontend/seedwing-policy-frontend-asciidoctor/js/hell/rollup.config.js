import { nodeResolve } from '@rollup/plugin-node-resolve';
import replace from '@rollup/plugin-replace';
import terser from '@rollup/plugin-terser';
import commonjs from '@rollup/plugin-commonjs';

export default {
  input: "main.js",
  output: [
    {
      file: "../debug/asciidoctor.js",
      format: "esm",
      compact: false,
    },
    {
      file: "../release/asciidoctor.js",
      format: "esm",
      compact: true,
      plugins: [
        terser()
      ]
    }
  ],
  plugins: [
    commonjs(),
    nodeResolve(),
    replace({
      "preventAssignment": true,
      "values": {
        "process.env.NODE_ENV": JSON.stringify('production')
      }
    }),
  ]
}