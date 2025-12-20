import { useState, useEffect, useCallback } from 'react';

interface UsePageDataOptions<T> {
  /** Function to fetch data */
  fetchFn: () => Promise<T[]>;
  /** Auto-refresh interval in ms (0 to disable) */
  refreshInterval?: number;
  /** Initial loading state */
  initialLoading?: boolean;
}

interface UsePageDataResult<T> {
  /** The fetched data */
  data: T[];
  /** Loading state */
  loading: boolean;
  /** Error message if any */
  error: string;
  /** Currently selected item */
  selected: T | null;
  /** Whether drawer/detail panel is open */
  drawerOpen: boolean;
  /** Reload data */
  reload: () => Promise<void>;
  /** Select an item and open drawer */
  select: (item: T) => void;
  /** Close drawer */
  closeDrawer: () => void;
  /** Clear selection */
  clearSelection: () => void;
  /** Set error manually */
  setError: (error: string) => void;
  /** Update data manually */
  setData: React.Dispatch<React.SetStateAction<T[]>>;
}

/**
 * Common hook for page data management with CRUD patterns
 */
export function usePageData<T>({
  fetchFn,
  refreshInterval = 0,
  initialLoading = true,
}: UsePageDataOptions<T>): UsePageDataResult<T> {
  const [data, setData] = useState<T[]>([]);
  const [loading, setLoading] = useState(initialLoading);
  const [error, setError] = useState('');
  const [selected, setSelected] = useState<T | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);

  const reload = useCallback(async () => {
    try {
      setLoading(true);
      const result = await fetchFn();
      setData(result);
      setError('');
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load data');
    } finally {
      setLoading(false);
    }
  }, [fetchFn]);

  const select = useCallback((item: T) => {
    setSelected(item);
    setDrawerOpen(true);
  }, []);

  const closeDrawer = useCallback(() => {
    setDrawerOpen(false);
  }, []);

  const clearSelection = useCallback(() => {
    setSelected(null);
    setDrawerOpen(false);
  }, []);

  // Initial load
  useEffect(() => {
    reload();
  }, [reload]);

  // Auto-refresh
  useEffect(() => {
    if (refreshInterval > 0) {
      const interval = setInterval(reload, refreshInterval);
      return () => clearInterval(interval);
    }
  }, [reload, refreshInterval]);

  return {
    data,
    loading,
    error,
    selected,
    drawerOpen,
    reload,
    select,
    closeDrawer,
    clearSelection,
    setError,
    setData,
  };
}

interface UseFormStateOptions<T> {
  initialValues: T;
  onSubmit: (values: T) => Promise<void>;
}

interface UseFormStateResult<T> {
  values: T;
  setValues: React.Dispatch<React.SetStateAction<T>>;
  setValue: <K extends keyof T>(key: K, value: T[K]) => void;
  submitting: boolean;
  error: string;
  submit: () => Promise<void>;
  reset: () => void;
}

/**
 * Simple form state management hook
 */
export function useFormState<T extends Record<string, unknown>>({
  initialValues,
  onSubmit,
}: UseFormStateOptions<T>): UseFormStateResult<T> {
  const [values, setValues] = useState<T>(initialValues);
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState('');

  const setValue = useCallback(<K extends keyof T>(key: K, value: T[K]) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  }, []);

  const submit = useCallback(async () => {
    try {
      setSubmitting(true);
      setError('');
      await onSubmit(values);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Operation failed');
      throw err;
    } finally {
      setSubmitting(false);
    }
  }, [values, onSubmit]);

  const reset = useCallback(() => {
    setValues(initialValues);
    setError('');
  }, [initialValues]);

  return {
    values,
    setValues,
    setValue,
    submitting,
    error,
    submit,
    reset,
  };
}
