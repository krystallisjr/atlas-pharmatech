import './globals.css'
import { DM_Sans } from 'next/font/google'
import { Providers } from '@/components/providers'

const dmSans = DM_Sans({ subsets: ['latin'], weight: ['400', '500', '600', '700'] })

export const metadata = {
  title: 'Atlas PharmaTech Marketplace',
  description: 'B2B Pharmaceutical Inventory Management and Trading Platform',
}

export default function RootLayout({
  children,
}: {
  children: React.ReactNode
}) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className={dmSans.className}>
        <Providers>
          {children}
        </Providers>
      </body>
    </html>
  )
}