'use client'

import { ReactNode } from 'react'
import { ThemeProvider, useTheme } from 'next-themes'
import { ToastContainer } from 'react-toastify'
import { AuthProvider } from '@/contexts/auth-context'

interface ProvidersProps {
  children: ReactNode
}

function ThemedToastContainer() {
  const { theme } = useTheme()

  return (
    <ToastContainer
      position="top-right"
      autoClose={5000}
      hideProgressBar={false}
      newestOnTop={false}
      closeOnClick
      rtl={false}
      pauseOnFocusLoss
      draggable
      pauseOnHover
      theme={theme === 'dark' ? 'dark' : 'light'}
    />
  )
}

export function Providers({ children }: ProvidersProps) {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      disableTransitionOnChange
      suppressHydrationWarning
    >
      <AuthProvider>
        {children}
        <ThemedToastContainer />
      </AuthProvider>
    </ThemeProvider>
  )
}