"use client";

import { useId } from "react";

const PATHS = [
  "M-40 90 C 180 10, 360 210, 640 70 S 980 190, 1320 40",
  "M-20 180 C 220 80, 420 280, 700 140 S 1040 40, 1400 160",
  "M 40 40 C 260 160, 480 -20, 760 120 S 1100 240, 1480 80",
  "M-60 260 C 160 140, 500 320, 820 180 S 1180 60, 1520 220",
  "M 80 320 C 300 200, 540 360, 880 240 S 1200 120, 1560 280",
  "M-10 140 C 240 240, 520 20, 800 180 S 1120 300, 1460 100",
  "M 120 220 C 340 40, 600 260, 900 80 S 1240 200, 1600 140",
  "M-30 300 C 200 360, 460 160, 740 280 S 1080 360, 1440 200",
];

export function BackgroundBeams() {
  const id = useId().replace(/:/g, "");
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden" aria-hidden>
      <svg className="absolute inset-0 h-full w-full" fill="none">
        <defs>
          {PATHS.map((_, i) => (
            <linearGradient
              key={i}
              id={`${id}-g${i}`}
              x1="0%"
              y1="0%"
              x2="100%"
              y2="0%"
            >
              <stop offset="0%" stopColor="#3E8E62" stopOpacity="0" />
              <stop offset="45%" stopColor={i % 2 ? "#9AE6B4" : "#3E8E62"} stopOpacity="0.7" />
              <stop offset="100%" stopColor="#3E8E62" stopOpacity="0" />
              <animate
                attributeName="x1"
                values="-80%;120%"
                dur={`${7 + i * 0.7}s`}
                repeatCount="indefinite"
              />
              <animate
                attributeName="x2"
                values="0%;200%"
                dur={`${7 + i * 0.7}s`}
                repeatCount="indefinite"
              />
            </linearGradient>
          ))}
        </defs>
        {PATHS.map((d, i) => (
          <path
            key={i}
            d={d}
            stroke={`url(#${id}-g${i})`}
            strokeWidth={i % 3 === 0 ? 1.6 : 1}
            vectorEffect="non-scaling-stroke"
          />
        ))}
      </svg>
    </div>
  );
}
