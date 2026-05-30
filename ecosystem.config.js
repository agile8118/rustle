module.exports = {
  apps: [
    {
      name: "rustle",
      script: "target/release/rustle",
      interpreter: "none",
      autorestart: true,
      watch: false,
      max_memory_restart: "1G",
    },
  ],
};
