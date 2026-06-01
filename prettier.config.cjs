/** @type {import('prettier').Config} */
module.exports = {
  plugins: ['prettier-plugin-svelte', 'prettier-plugin-organize-imports'],
  singleQuote: true,
  trailingComma: 'none',
  overrides: [
    {
      files: '*.svelte',
      options: {
        parser: 'svelte'
      }
    }
  ]
};
