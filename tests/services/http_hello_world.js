const http = require("node:http");

const server = http.createServer((req, res) => {
  res.writeHead(200, { "Content-Type": "text/plain" });
  res.end("Hello, world!");
});
process.once("SIGTERM", () => server.close());
server.listen(parseInt(process.env.PORT || "8080", 10), "::1");
