import React from 'react';

// =============================================================================
// Stat Card - Compact metric display
// =============================================================================

type StatColor = 'primary' | 'secondary' | 'success' | 'warning' | 'danger' | 'info' | 'muted';

const COLOR_MAP: Record<StatColor, { bg: string; border: string; text: string }> = {
  primary: { bg: 'rgba(6, 182, 212, 0.15)', border: 'rgba(6, 182, 212, 0.25)', text: '#06b6d4' },
  secondary: { bg: 'rgba(139, 92, 246, 0.15)', border: 'rgba(139, 92, 246, 0.25)', text: '#8b5cf6' },
  success: { bg: 'rgba(34, 197, 94, 0.15)', border: 'rgba(34, 197, 94, 0.25)', text: '#22c55e' },
  warning: { bg: 'rgba(245, 158, 11, 0.15)', border: 'rgba(245, 158, 11, 0.25)', text: '#f59e0b' },
  danger: { bg: 'rgba(239, 68, 68, 0.15)', border: 'rgba(239, 68, 68, 0.25)', text: '#ef4444' },
  info: { bg: 'rgba(59, 130, 246, 0.15)', border: 'rgba(59, 130, 246, 0.25)', text: '#3b82f6' },
  muted: { bg: 'rgba(107, 114, 128, 0.15)', border: 'rgba(107, 114, 128, 0.25)', text: '#6b7280' },
};

interface StatCardProps {
  label: string;
  value: string | number;
  icon?: React.ReactNode;
  color?: StatColor;
  compact?: boolean;
}

export function StatCard({ label, value, icon, color = 'primary', compact = true }: StatCardProps) {
  const colors = COLOR_MAP[color];

  if (compact) {
    return (
      <div
        className="flex items-center gap-3 p-3 rounded-lg"
        style={{
          background: 'var(--bg-secondary)',
          border: '1px solid var(--border-primary)',
        }}
      >
        {icon && (
          <div
            className="flex items-center justify-center w-8 h-8 rounded-md flex-shrink-0"
            style={{
              background: colors.bg,
              border: `1px solid ${colors.border}`,
              color: colors.text,
            }}
          >
            {icon}
          </div>
        )}
        <div className="flex flex-col min-w-0">
          <span className="text-lg font-semibold" style={{ color: 'var(--text-primary)' }}>
            {value}
          </span>
          <span className="text-xs truncate" style={{ color: 'var(--text-muted)' }}>
            {label}
          </span>
        </div>
      </div>
    );
  }

  // Full size variant
  return (
    <div className="stat-card">
      {icon && (
        <div
          className="stat-icon mb-2"
          style={{
            background: colors.bg,
            border: `1px solid ${colors.border}`,
            color: colors.text,
          }}
        >
          {icon}
        </div>
      )}
      <span className="stat-value">{value}</span>
      <span className="stat-label">{label}</span>
    </div>
  );
}

// =============================================================================
// Badge - Status indicators
// =============================================================================

interface BadgeProps {
  children: React.ReactNode;
  variant?: StatColor;
  size?: 'sm' | 'md';
}

export function Badge({ children, variant = 'muted', size = 'sm' }: BadgeProps) {
  const colors = COLOR_MAP[variant];
  const sizeClasses = size === 'sm' ? 'px-2 py-0.5 text-xs' : 'px-3 py-1 text-sm';

  return (
    <span
      className={`inline-flex items-center rounded-full font-medium ${sizeClasses}`}
      style={{
        background: colors.bg,
        color: colors.text,
        border: `1px solid ${colors.border}`,
      }}
    >
      {children}
    </span>
  );
}

// =============================================================================
// StatusDot - Live status indicator
// =============================================================================

interface StatusDotProps {
  status: 'online' | 'offline' | 'warning' | 'idle';
  pulse?: boolean;
}

export function StatusDot({ status, pulse = true }: StatusDotProps) {
  const colorMap = {
    online: '#22c55e',
    offline: '#ef4444',
    warning: '#f59e0b',
    idle: '#6b7280',
  };

  return (
    <span
      className={`inline-block w-2 h-2 rounded-full ${pulse && status === 'online' ? 'animate-pulse' : ''}`}
      style={{ background: colorMap[status] }}
    />
  );
}

// =============================================================================
// Empty State - For when there's no data
// =============================================================================

interface EmptyStateProps {
  icon?: React.ReactNode;
  title: string;
  description?: string;
  action?: React.ReactNode;
}

export function EmptyState({ icon, title, description, action }: EmptyStateProps) {
  return (
    <div className="flex flex-col items-center justify-center py-12 text-center">
      {icon && (
        <div className="mb-4 opacity-30" style={{ color: 'var(--text-muted)' }}>
          {icon}
        </div>
      )}
      <h3 className="text-lg font-medium mb-1" style={{ color: 'var(--text-primary)' }}>
        {title}
      </h3>
      {description && (
        <p className="text-sm mb-4" style={{ color: 'var(--text-muted)' }}>
          {description}
        </p>
      )}
      {action}
    </div>
  );
}

// =============================================================================
// Loading Spinner
// =============================================================================

interface SpinnerProps {
  size?: 'sm' | 'md' | 'lg';
  label?: string;
}

export function Spinner({ size = 'md', label }: SpinnerProps) {
  const sizeClasses = {
    sm: 'w-4 h-4',
    md: 'w-6 h-6',
    lg: 'w-8 h-8',
  };

  return (
    <div className="flex items-center gap-3" style={{ color: 'var(--text-muted)' }}>
      <svg
        className={`animate-spin ${sizeClasses[size]}`}
        fill="none"
        stroke="currentColor"
        viewBox="0 0 24 24"
      >
        <path
          strokeLinecap="round"
          strokeLinejoin="round"
          strokeWidth={2}
          d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
        />
      </svg>
      {label && <span>{label}</span>}
    </div>
  );
}

// =============================================================================
// Page Header
// =============================================================================

interface PageHeaderProps {
  title: string;
  subtitle?: string;
  actions?: React.ReactNode;
  badge?: React.ReactNode;
}

export function PageHeader({ title, subtitle, actions, badge }: PageHeaderProps) {
  return (
    <div className="flex items-center justify-between mb-6">
      <div>
        <h1 className="text-2xl font-bold flex items-center gap-3" style={{ color: 'var(--text-primary)' }}>
          {title}
          {badge}
        </h1>
        {subtitle && (
          <p className="text-sm mt-1" style={{ color: 'var(--text-muted)' }}>
            {subtitle}
          </p>
        )}
      </div>
      {actions && <div className="flex items-center gap-3">{actions}</div>}
    </div>
  );
}

// =============================================================================
// Card
// =============================================================================

interface CardProps {
  children: React.ReactNode;
  title?: string;
  actions?: React.ReactNode;
  className?: string;
  padding?: 'none' | 'sm' | 'md' | 'lg';
}

export function Card({ children, title, actions, className = '', padding = 'md' }: CardProps) {
  const paddingClasses = {
    none: '',
    sm: 'p-3',
    md: 'p-4',
    lg: 'p-6',
  };

  return (
    <div className={`card ${paddingClasses[padding]} ${className}`}>
      {(title || actions) && (
        <div className="flex items-center justify-between mb-4">
          {title && (
            <h3 className="font-semibold" style={{ color: 'var(--text-primary)' }}>
              {title}
            </h3>
          )}
          {actions}
        </div>
      )}
      {children}
    </div>
  );
}

// =============================================================================
// Field - For detail views
// =============================================================================

interface FieldProps {
  label: string;
  value: React.ReactNode;
  mono?: boolean;
}

export function Field({ label, value, mono = false }: FieldProps) {
  return (
    <div className="py-2" style={{ borderBottom: '1px solid var(--border-primary)' }}>
      <dt className="text-xs font-medium mb-1" style={{ color: 'var(--text-muted)' }}>
        {label}
      </dt>
      <dd
        className={`text-sm ${mono ? 'font-mono' : ''}`}
        style={{ color: 'var(--text-primary)' }}
      >
        {value || '-'}
      </dd>
    </div>
  );
}

// =============================================================================
// Alert
// =============================================================================

interface AlertProps {
  variant: 'success' | 'warning' | 'danger' | 'info';
  title?: string;
  children: React.ReactNode;
  onDismiss?: () => void;
}

export function Alert({ variant, title, children, onDismiss }: AlertProps) {
  return (
    <div className={`alert alert-${variant} flex items-start gap-3`}>
      <div className="flex-1">
        {title && <p className="font-medium">{title}</p>}
        <div className="text-sm opacity-90">{children}</div>
      </div>
      {onDismiss && (
        <button onClick={onDismiss} className="opacity-70 hover:opacity-100">
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      )}
    </div>
  );
}

// =============================================================================
// Stat Grid - For dashboard-style layouts
// =============================================================================

interface StatGridProps {
  children: React.ReactNode;
  columns?: 2 | 3 | 4 | 6;
}

export function StatGrid({ children, columns = 6 }: StatGridProps) {
  const colClasses = {
    2: 'grid-cols-2',
    3: 'grid-cols-2 md:grid-cols-3',
    4: 'grid-cols-2 md:grid-cols-4',
    6: 'grid-cols-2 md:grid-cols-3 lg:grid-cols-6',
  };

  return <div className={`grid gap-4 ${colClasses[columns]}`}>{children}</div>;
}
