// webpack.config.prod.js

const path = require('path');
const webpack = require('webpack');
const { WebpackManifestPlugin } = require('webpack-manifest-plugin');
const TerserPlugin = require('terser-webpack-plugin');
const CssMinimizerPlugin = require('css-minimizer-webpack-plugin');
const MiniCssExtractPlugin = require('mini-css-extract-plugin');
const { BundleAnalyzerPlugin } = require('webpack-bundle-analyzer');
const CompressionPlugin = require('compression-webpack-plugin');
const zlib = require('zlib');

module.exports = (env) => ({
  mode: 'production',
  target: 'browserslist:modern',
  devtool: 'hidden-source-map',

  entry: {
    main: './src/index.tsx',
    sw: './src/service-worker.ts',
  },

  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'static/js/[name].[contenthash:8].js',
    chunkFilename: 'static/js/[name].[contenthash:8].chunk.js',
    publicPath: 'https://cdn.scoria.ai/',
    crossOriginLoading: 'anonymous',
  },

  resolve: {
    extensions: ['.ts', '.tsx', '.js', '.jsx', '.json', '.wasm'],
    alias: {
      '@solana': path.resolve(__dirname, 'node_modules/@solana'),
      '@crypto': path.resolve(__dirname, 'src/lib/crypto'),
      '@zk': path.resolve(__dirname, 'src/lib/zk'),
    },
    fallback: {
      'crypto': require.resolve('crypto-browserify'),
      'stream': require.resolve('stream-browserify'),
    },
  },

  module: {
    rules: [
      {
        test: /\.(ts|tsx)$/,
        exclude: /node_modules/,
        use: [
          {
            loader: 'ts-loader',
            options: {
              configFile: 'tsconfig.prod.json',
              transpileOnly: false,
              experimentalWatchApi: true,
            },
          },
        ],
      },
      {
        test: /\.module\.(scss|css)$/,
        use: [
          MiniCssExtractPlugin.loader,
          {
            loader: 'css-loader',
            options: {
              modules: {
                localIdentName: '[hash:base64:8]',
              },
              importLoaders: 2,
            },
          },
          'postcss-loader',
          'sass-loader',
        ],
      },
      {
        test: /\.(png|jpe?g|gif|webp|avif)$/i,
        type: 'asset/resource',
        generator: {
          filename: 'static/media/[name].[contenthash:8][ext]',
        },
      },
      {
        test: /\.wasm$/,
        type: 'asset/resource',
        generator: {
          filename: 'static/wasm/[name].[contenthash:8][ext]',
        },
      },
      {
        test: /\.node$/,
        loader: 'native-ext-loader',
      },
    ],
  },

  optimization: {
    minimize: true,
    minimizer: [
      new TerserPlugin({
        parallel: true,
        terserOptions: {
          ecma: 2020,
          compress: {
            warnings: false,
            drop_console: true,
            pure_funcs: ['console.debug'],
          },
          format: {
            comments: false,
          },
        },
        extractComments: false,
      }),
      new CssMinimizerPlugin({
        minimizerOptions: {
          preset: [
            'default',
            {
              discardComments: { removeAll: true },
              colormin: false,
            },
          ],
        },
      }),
    ],
    splitChunks: {
      chunks: 'all',
      maxInitialRequests: 20,
      maxAsyncRequests: 20,
      minSize: 40000,
      cacheGroups: {
        solana: {
          test: /[\\/]node_modules[\\/]@solana[\\/]/,
          priority: 20,
          reuseExistingChunk: true,
        },
        react: {
          test: /[\\/]node_modules[\\/](react|react-dom)[\\/]/,
          priority: 10,
          reuseExistingChunk: true,
        },
        lib: {
          test: /[\\/]src[\\/]lib[\\/]/,
          priority: 5,
          reuseExistingChunk: true,
        },
      },
    },
    runtimeChunk: 'single',
  },

  plugins: [
    new webpack.EnvironmentPlugin({
      NODE_ENV: 'production',
      SCORIA_API_ENDPOINT: 'https://api.scoria.ai/v2',
      BLOCKCHAIN_NETWORK: 'mainnet-beta',
    }),
    new MiniCssExtractPlugin({
      filename: 'static/css/[name].[contenthash:8].css',
      chunkFilename: 'static/css/[name].[contenthash:8].chunk.css',
    }),
    new CompressionPlugin({
      algorithm: 'brotliCompress',
      test: /\.(js|css|html|svg|wasm)$/,
      threshold: 10240,
      minRatio: 0.8,
      compressionOptions: {
        params: {
          [zlib.constants.BROTLI_PARAM_QUALITY]: 11,
        },
      },
    }),
    new WebpackManifestPlugin({
      fileName: 'asset-manifest.json',
      generate: (seed, files) => ({
        files: files.reduce((manifest, file) => {
          manifest[file.name] = file.path;
          return manifest;
        }, seed),
      }),
    }),
    new BundleAnalyzerPlugin({
      analyzerMode: 'static',
      reportFilename: 'bundle-analysis.html',
      openAnalyzer: false,
    }),
    new webpack.SourceMapDevToolPlugin({
      test: /\.(js|css)($|\?)/i,
      filename: '[file].map',
      append: `\n//# sourceMappingURL=https://cdn.scoria.ai/[url]`,
      module: true,
      columns: false,
    }),
  ],

  performance: {
    maxAssetSize: 500000,
    maxEntrypointSize: 500000,
    hints: 'error',
    assetFilter: (asset) => {
      return !/\.(map|wasm|worker\.js)$/.test(asset.name);
    },
  },
});
