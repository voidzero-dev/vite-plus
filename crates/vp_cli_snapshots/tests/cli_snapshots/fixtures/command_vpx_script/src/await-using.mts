await using resource = {
  async [Symbol.asyncDispose]() {
    console.log('resource disposed');
  },
};

console.log('resource active');
