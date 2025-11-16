import React from 'react';

interface AtlasLogoProps {
  className?: string;
  size?: number;
}

export function AtlasLogo({ className = '', size = 32 }: AtlasLogoProps) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 100 100"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
      className={className}
    >
      {/* Blue orbital rings (particle detector style) */}
      {/* Main vertical ellipse */}
      <ellipse
        cx="50"
        cy="35"
        rx="12"
        ry="24"
        fill="none"
        stroke="#1E88E5"
        strokeWidth="3"
        opacity="0.4"
      />

      {/* Diagonal ellipse 1 */}
      <ellipse
        cx="50"
        cy="35"
        rx="24"
        ry="12"
        fill="none"
        stroke="#0D47A1"
        strokeWidth="3.5"
        transform="rotate(-35 50 35)"
      />

      {/* Diagonal ellipse 2 */}
      <ellipse
        cx="50"
        cy="35"
        rx="24"
        ry="12"
        fill="none"
        stroke="#1976D2"
        strokeWidth="3.5"
        transform="rotate(35 50 35)"
      />

      {/* Central blue sphere/globe */}
      <circle
        cx="50"
        cy="35"
        r="10"
        fill="#1E88E5"
      />

      {/* Atlas figure - black silhouette holding the sphere */}
      {/* Head */}
      <circle
        cx="50"
        cy="58"
        r="6"
        fill="#000000"
      />

      {/* Left arm reaching up */}
      <path
        d="M 47 62 Q 42 58 38 52 Q 35 46 38 42"
        fill="none"
        stroke="#000000"
        strokeWidth="5"
        strokeLinecap="round"
      />

      {/* Right arm reaching up */}
      <path
        d="M 53 62 Q 58 58 62 52 Q 65 46 62 42"
        fill="none"
        stroke="#000000"
        strokeWidth="5"
        strokeLinecap="round"
      />

      {/* Torso */}
      <path
        d="M 50 64 L 46 82 M 50 64 L 54 82"
        stroke="#000000"
        strokeWidth="6"
        strokeLinecap="round"
      />

      {/* Left leg (straight) */}
      <path
        d="M 46 82 L 42 96"
        stroke="#000000"
        strokeWidth="5.5"
        strokeLinecap="round"
      />

      {/* Right leg (bent/dynamic) */}
      <path
        d="M 54 82 Q 56 88 60 92 L 62 96"
        fill="none"
        stroke="#000000"
        strokeWidth="5.5"
        strokeLinecap="round"
      />
    </svg>
  );
}

export default AtlasLogo;
