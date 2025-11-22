import React from 'react';
import Image from 'next/image';

interface AtlasLogoProps {
  className?: string;
  size?: number;
}

export function AtlasLogo({ className = '', size = 32 }: AtlasLogoProps) {
  return (
    <Image
      src="/logo.png"
      alt="Atlas PharmaTech"
      width={size}
      height={size}
      className={className}
      priority
    />
  );
}

export default AtlasLogo;
