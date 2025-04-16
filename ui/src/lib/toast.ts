import { toast as sonnerToast, type ExternalToast } from 'sonner';

type ToastType = 'success' | 'error' | 'info' | 'warning';

interface ToastOptions {
  duration?: number;
  position?: 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right' | 'top-center' | 'bottom-center' | 'top' | 'bottom';
  action?: {
    label: string;
    onClick: () => void;
  };
}

export const toast = {
  success: (message: string, options?: ExternalToast) => {
    sonnerToast.success(message, options);
  },
  error: (message: string, options?: ExternalToast) => {
    sonnerToast.error(message, options);
  },
  info: (message: string, options?: ExternalToast) => {
    sonnerToast.info(message, options);
  },
  warning: (message: string, options?: ExternalToast) => {
    sonnerToast.warning(message, options);
  },
  promise: <T>(
    promise: Promise<T>,
    {
      loading,
      success,
      error,
    }: {
      loading: string;
      success: string;
      error: string;
    }
  ) => {
    return sonnerToast.promise(promise, {
      loading,
      success,
      error,
    });
  },
  dismiss: (toastId?: string) => {
    sonnerToast.dismiss(toastId);
  },
  custom: (message: React.ReactNode, options?: ExternalToast) => {
    sonnerToast(message, options);
  },
}; 