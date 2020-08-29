const path = require("path");
const CopyPlugin = require("copy-webpack-plugin");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");

const dist = path.resolve(__dirname, "dist");

module.exports = {
  mode: "production",
  entry: {
    index: "./js/index.js"
  },
  output: {
    path: dist,
    filename: "[name].js"
  },
  devServer: {
    contentBase: dist,
  },
  devtool: 'source-map',
  plugins: [
    new CopyPlugin({
      patterns: [
        path.resolve(__dirname, "static"),
        { from: path.resolve(__dirname, "node_modules/@mdi/font/fonts"), to: "fonts" },
        { from: path.resolve(__dirname, "node_modules/@mdi/font/css"), to: "css" },
      ],
    }),

    new WasmPackPlugin({
      crateDirectory: __dirname,
    }),
  ]
};
