// webpack.config.dev.js

const path = require('path');
const webpack = require('webpack');
const ReactRefreshWebpackPlugin = require('@pmmmwh/react-refresh-webpack-plugin');
const { createProxyMiddleware } = require('http-proxy-middleware');

module.exports = {
  mode: 'development',
  target: 'web',
  devtool: 'eval-cheap-module-source-map',

  entry: {
    main: [
      'webpack-dev-server/client?https://localhost:3000',
      'webpack/hot/dev-server',
      './src/index.tsx'
    ]
  },

  output: {
    path: path.resolve(__dirname, 'dist'),
    filename: 'static/js/[name].bundle.js',
    chunkFilename: 'static/js/[name].chunk.js',
    publicPath: '/',
    assetModuleFilename: 'static/media/[name][ext]'
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
      'fs': false,
      'path': false
    }
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
              transpileOnly: true,
              experimentalWatchApi: true,
              compilerOptions: {
                isolatedModules: true
              }
            }
          }
        ]
      },
      {
        test: /\.module\.(scss|css)$/,
        use: [
          'style-loader',
          {
            loader: 'css-loader',
            options: {
              modules: {
                localIdentName: '[path][name]__[local]'
              },
              sourceMap: true,
              importLoaders: 2
            }
          },
          'postcss-loader',
          'sass-loader'
        ]
      },
      {
        test: /\.(png|jpe?g|gif|webp|avif)$/i,
        type: 'asset/resource'
      },
      {
        test: /\.wasm$/,
        type: 'asset/resource'
      }
    ]
  },

  plugins: [
    new webpack.HotModuleReplacementPlugin(),
    new ReactRefreshWebpackPlugin({
      overlay: {
        sockIntegration: 'wds',
        module: path.resolve(__dirname, 'src/ErrorOverlay.tsx')
      }
    }),
    new webpack.EnvironmentPlugin({
      NODE_ENV: 'development',
      SCORIA_API_ENDPOINT: 'https://dev.api.scoria.ai/v2',
      BLOCKCHAIN_NETWORK: 'devnet'
    }),
    new webpack.SourceMapDevToolPlugin({
      test: /\.(js|css)($|\?)/i,
      filename: '[file].map',
      module: true,
      columns: true
    })
  ],

  devServer: {
    host: 'localhost',
    port: 3000,
    hot: true,
    open: true,
    historyApiFallback: true,
    client: {
      overlay: {
        errors: true,
        warnings: false
      },
      logging: 'warn',
      progress: true
    },
    static: {
      directory: path.join(__dirname, 'public'),
      publicPath: '/'
    },
    proxy: {
      '/api': {
        target: 'https://dev.api.scoria.ai',
        changeOrigin: true,
        pathRewrite: {'^/api': ''}
      },
      '/rpc': {
        target: 'https://api.devnet.solana.com',
        changeOrigin: true,
        pathRewrite: {'^/rpc': ''}
      }
    },
    allowedHosts: 'all',
    compress: true,
    headers: {
      'Access-Control-Allow-Origin': '*',
      'Cross-Origin-Embedder-Policy': 'require-corp',
      'Cross-Origin-Opener-Policy': 'same-origin'
    }
  },

  experiments: {
    asyncWebAssembly: true,
    topLevelAwait: true
  },

  performance: {
    hints: 'warning',
    maxAssetSize: 1048576,
    maxEntrypointSize: 1048576
  },

  optimization: {
    removeAvailableModules: false,
    removeEmptyChunks: false,
    splitChunks: false,
    minimize: false,
    minimizer: []
  }
};
