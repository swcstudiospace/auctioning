/** @type {import('next').NextConfig} */
const nextConfig = {
  // Static-first marketing site: every page prerenders, no server functions.
  output: "export",
  trailingSlash: true,
};

export default nextConfig;
