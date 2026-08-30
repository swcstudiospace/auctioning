const BACKEND = (process.env.AUCTIONING_INTERNAL_API_URL || "http://127.0.0.1:8000").replace(
  /\/$/,
  "",
);

/** @type {import('next').NextConfig} */
const nextConfig = {
  reactStrictMode: true,
  async rewrites() {
    return [
      { source: "/v1/:path*", destination: `${BACKEND}/v1/:path*` },
      { source: "/healthz", destination: `${BACKEND}/healthz` },
    ];
  },
};
export default nextConfig;
