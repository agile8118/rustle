module.exports = {
  apps: [
    {
      name: "rustle",
      script: "./env.sh",
      args: "./server",
      interpreter: "none",
      autorestart: true,
      watch: false,
      max_memory_restart: "1G",
    },
  ],
};
