const BACKEND = (process.env.AUCTIONING_INTERNAL_API_URL || "http://127.0.0.1:8000").replace(
  /\/$/,
  "",
);

if (process.env.VERCEL_ENV === "production") {
  const loopback = /^(https?:\/\/)?(127\.0\.0\.1|localhost)(:|\/|$)/i.test(BACKEND);
  if (loopback) {
    console.warn(
      "[auctioning] AUCTIONING_INTERNAL_API_URL is loopback in production; /v1 rewrites will 404. Set it to the public Shuttle API URL.",
    );
  }
}

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
