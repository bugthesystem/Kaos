import { useEffect, type ReactNode } from 'react';

interface DrawerProps {
  open: boolean;
  onClose: () => void;
  title: string;
  children: ReactNode;
  footer?: ReactNode;
  width?: 'sm' | 'md' | 'lg' | 'xl';
}

const widthClasses = {
  sm: '',
  md: 'drawer-md',
  lg: 'drawer-lg',
  xl: 'drawer-xl',
};

export function Drawer({ open, onClose, title, children, footer, width = 'md' }: DrawerProps) {
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && open) {
        onClose();
      }
    };

    document.addEventListener('keydown', handleEscape);
    return () => document.removeEventListener('keydown', handleEscape);
  }, [open, onClose]);

  useEffect(() => {
    if (open) {
      document.body.style.overflow = 'hidden';
    } else {
      document.body.style.overflow = '';
    }
    return () => {
      document.body.style.overflow = '';
    };
  }, [open]);

  return (
    <>
      {/* Overlay */}
      <div
        className={`drawer-overlay ${open ? 'open' : ''}`}
        onClick={onClose}
      />

      {/* Drawer Panel */}
      <div className={`drawer ${open ? 'open' : ''} ${widthClasses[width]}`}>
        {/* Header */}
        <div className="drawer-header">
          <h2 className="drawer-title">{title}</h2>
          <button onClick={onClose} className="drawer-close">
            <CloseIcon className="w-5 h-5" />
          </button>
        </div>

        {/* Body */}
        <div className="drawer-body">
          {children}
        </div>

        {/* Footer */}
        {footer && (
          <div className="drawer-footer">
            {footer}
          </div>
        )}
      </div>
    </>
  );
}

function CloseIcon({ className, style }: { className?: string; style?: React.CSSProperties }) {
  return (
    <svg className={className} style={style} fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
    </svg>
  );
}

// Field component for consistent detail display
interface FieldProps {
  label: string;
  children: ReactNode;
  mono?: boolean;
}

export function Field({ label, children, mono }: FieldProps) {
  return (
    <div className="py-3 border-b" style={{ borderColor: 'var(--border-primary)' }}>
      <dt className="text-xs font-medium uppercase tracking-wide mb-1" style={{ color: 'var(--text-muted)' }}>
        {label}
      </dt>
      <dd className={`text-sm ${mono ? 'font-mono' : ''}`} style={{ color: 'var(--text-primary)' }}>
        {children || <span style={{ color: 'var(--text-muted)' }}>-</span>}
      </dd>
    </div>
  );
}

// Section component for grouping fields
interface SectionProps {
  title: string;
  children: ReactNode;
}

export function Section({ title, children }: SectionProps) {
  return (
    <div className="mb-6">
      <h3 className="text-sm font-semibold mb-3" style={{ color: 'var(--text-primary)' }}>
        {title}
      </h3>
      <div className="card p-4">
        {children}
      </div>
    </div>
  );
}
