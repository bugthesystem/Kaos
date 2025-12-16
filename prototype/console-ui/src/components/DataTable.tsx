import { useState, useMemo, useRef, useEffect, type ReactNode } from 'react';

export interface Column<T> {
  key: string;
  header: string;
  width?: string;
  align?: 'left' | 'center' | 'right';
  filterable?: boolean;
  sortable?: boolean;
  render?: (item: T) => ReactNode;
}

interface DataTableProps<T> {
  data: T[];
  columns: Column<T>[];
  keyField: keyof T;
  onRowClick?: (item: T) => void;
  selectedId?: string | number | null;
  loading?: boolean;
  emptyMessage?: string;
  searchable?: boolean;
  searchPlaceholder?: string;
  searchFields?: (keyof T)[];
  pagination?: boolean;
  pageSize?: number;
  compact?: boolean;
}

type SortDirection = 'asc' | 'desc' | null;

export function DataTable<T extends Record<string, any>>({
  data,
  columns,
  keyField,
  onRowClick,
  selectedId,
  loading = false,
  emptyMessage = 'No data available',
  searchable = false,
  searchPlaceholder = 'Search...',
  searchFields = [],
  pagination = true,
  pageSize = 10,
  compact = true,
}: DataTableProps<T>) {
  const [globalSearch, setGlobalSearch] = useState('');
  const [columnFilters, setColumnFilters] = useState<Record<string, string>>({});
  const [activeFilter, setActiveFilter] = useState<string | null>(null);
  const [sortColumn, setSortColumn] = useState<string | null>(null);
  const [sortDirection, setSortDirection] = useState<SortDirection>(null);
  const [page, setPage] = useState(1);
  const filterRef = useRef<HTMLDivElement>(null);

  // Close filter dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (filterRef.current && !filterRef.current.contains(event.target as Node)) {
        setActiveFilter(null);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  // Filter by global search
  const globalFiltered = useMemo(() => {
    if (!globalSearch || searchFields.length === 0) return data;
    const query = globalSearch.toLowerCase();
    return data.filter((item) =>
      searchFields.some((field) => {
        const value = item[field];
        return value && String(value).toLowerCase().includes(query);
      })
    );
  }, [data, globalSearch, searchFields]);

  // Filter by column filters
  const columnFiltered = useMemo(() => {
    const activeFilters = Object.entries(columnFilters).filter(([_, v]) => v);
    if (activeFilters.length === 0) return globalFiltered;

    return globalFiltered.filter((item) =>
      activeFilters.every(([key, filterValue]) => {
        const value = item[key];
        if (value === undefined || value === null) return false;
        return String(value).toLowerCase().includes(filterValue.toLowerCase());
      })
    );
  }, [globalFiltered, columnFilters]);

  // Sort data
  const sortedData = useMemo(() => {
    if (!sortColumn || !sortDirection) return columnFiltered;

    return [...columnFiltered].sort((a, b) => {
      const aVal = a[sortColumn];
      const bVal = b[sortColumn];

      if (aVal === bVal) return 0;
      if (aVal === null || aVal === undefined) return 1;
      if (bVal === null || bVal === undefined) return -1;

      const comparison = typeof aVal === 'number' && typeof bVal === 'number'
        ? aVal - bVal
        : String(aVal).localeCompare(String(bVal));

      return sortDirection === 'asc' ? comparison : -comparison;
    });
  }, [columnFiltered, sortColumn, sortDirection]);

  // Paginate
  const paginatedData = useMemo(() => {
    if (!pagination) return sortedData;
    const start = (page - 1) * pageSize;
    return sortedData.slice(start, start + pageSize);
  }, [sortedData, page, pageSize, pagination]);

  const totalPages = Math.ceil(sortedData.length / pageSize);

  const handleGlobalSearch = (value: string) => {
    setGlobalSearch(value);
    setPage(1);
  };

  const handleColumnFilter = (key: string, value: string) => {
    setColumnFilters((prev) => ({ ...prev, [key]: value }));
    setPage(1);
  };

  const handleSort = (key: string) => {
    if (sortColumn === key) {
      if (sortDirection === 'asc') setSortDirection('desc');
      else if (sortDirection === 'desc') {
        setSortColumn(null);
        setSortDirection(null);
      }
    } else {
      setSortColumn(key);
      setSortDirection('asc');
    }
  };

  const clearAllFilters = () => {
    setColumnFilters({});
    setGlobalSearch('');
    setPage(1);
  };

  const hasActiveFilters = Object.values(columnFilters).some(v => v) || globalSearch;
  const activeFilterCount = Object.values(columnFilters).filter(v => v).length + (globalSearch ? 1 : 0);

  if (loading) {
    return (
      <div className="table-container">
        <div className="p-8 text-center">
          <div className="spinner mx-auto mb-3" />
          <p style={{ color: 'var(--text-muted)' }}>Loading...</p>
        </div>
      </div>
    );
  }

  return (
    <div className="data-table-wrapper">
      {/* Toolbar */}
      <div className="data-table-toolbar">
        <div className="flex items-center gap-3">
          {searchable && (
            <div className="data-table-search">
              <SearchIcon />
              <input
                type="text"
                value={globalSearch}
                onChange={(e) => handleGlobalSearch(e.target.value)}
                placeholder={searchPlaceholder}
              />
              {globalSearch && (
                <button onClick={() => setGlobalSearch('')} className="clear-btn">
                  <CloseIcon />
                </button>
              )}
            </div>
          )}
          {hasActiveFilters && (
            <button onClick={clearAllFilters} className="clear-filters-btn">
              <CloseIcon />
              Clear {activeFilterCount} filter{activeFilterCount > 1 ? 's' : ''}
            </button>
          )}
        </div>
        <div className="data-table-info">
          {sortedData.length} {sortedData.length === 1 ? 'row' : 'rows'}
          {data.length !== sortedData.length && ` (${data.length} total)`}
        </div>
      </div>

      {/* Table */}
      <div className="table-container overflow-hidden">
        <div className="overflow-x-auto">
          <table className={`data-table ${compact ? 'compact' : ''}`}>
            <thead>
              <tr>
                {columns.map((col) => {
                  const isFilterable = col.filterable !== false;
                  const isSortable = col.sortable !== false;
                  const isFiltered = columnFilters[col.key];
                  const isSorted = sortColumn === col.key;

                  return (
                    <th
                      key={col.key}
                      style={{ width: col.width }}
                      className={`${isSortable ? 'sortable' : ''} ${isSorted ? 'sorted' : ''}`}
                    >
                      <div className="th-content">
                        <div
                          className="th-label"
                          onClick={() => isSortable && handleSort(col.key)}
                        >
                          <span>{col.header}</span>
                          {isSortable && (
                            <span className="sort-indicator">
                              {isSorted ? (
                                sortDirection === 'asc' ? <SortAscIcon /> : <SortDescIcon />
                              ) : (
                                <SortIcon />
                              )}
                            </span>
                          )}
                        </div>
                        {isFilterable && (
                          <div className="th-filter" ref={activeFilter === col.key ? filterRef : null}>
                            <button
                              className={`filter-btn ${isFiltered ? 'active' : ''}`}
                              onClick={(e) => {
                                e.stopPropagation();
                                setActiveFilter(activeFilter === col.key ? null : col.key);
                              }}
                            >
                              <FilterIcon />
                            </button>
                            {activeFilter === col.key && (
                              <div className="filter-dropdown">
                                <input
                                  type="text"
                                  value={columnFilters[col.key] || ''}
                                  onChange={(e) => handleColumnFilter(col.key, e.target.value)}
                                  placeholder={`Filter ${col.header}...`}
                                  autoFocus
                                  onClick={(e) => e.stopPropagation()}
                                />
                                {columnFilters[col.key] && (
                                  <button
                                    onClick={(e) => {
                                      e.stopPropagation();
                                      handleColumnFilter(col.key, '');
                                    }}
                                    className="clear-filter"
                                  >
                                    Clear
                                  </button>
                                )}
                              </div>
                            )}
                          </div>
                        )}
                      </div>
                    </th>
                  );
                })}
              </tr>
            </thead>
            <tbody>
              {paginatedData.length === 0 ? (
                <tr>
                  <td
                    colSpan={columns.length}
                    className="text-center py-12"
                    style={{ color: 'var(--text-muted)' }}
                  >
                    <div className="empty-state-icon mx-auto mb-3">
                      <EmptyIcon />
                    </div>
                    {hasActiveFilters ? 'No matching results' : emptyMessage}
                  </td>
                </tr>
              ) : (
                paginatedData.map((item) => {
                  const id = item[keyField];
                  const isSelected = selectedId !== undefined && selectedId === id;

                  return (
                    <tr
                      key={String(id)}
                      className={`${isSelected ? 'selected' : ''} ${onRowClick ? 'clickable' : ''}`}
                      onClick={() => onRowClick?.(item)}
                    >
                      {columns.map((col) => (
                        <td
                          key={col.key}
                          style={{ textAlign: col.align || 'left' }}
                        >
                          {col.render ? col.render(item) : item[col.key]}
                        </td>
                      ))}
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>

      {/* Pagination */}
      {pagination && totalPages > 1 && (
        <div className="data-table-pagination">
          <p className="pagination-info-text">
            {((page - 1) * pageSize) + 1}-{Math.min(page * pageSize, sortedData.length)} of {sortedData.length}
          </p>
          <div className="pagination">
            <button
              className="pagination-btn"
              onClick={() => setPage(1)}
              disabled={page === 1}
              title="First page"
            >
              <ChevronDoubleLeftIcon />
            </button>
            <button
              className="pagination-btn"
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              disabled={page === 1}
              title="Previous page"
            >
              <ChevronLeftIcon />
            </button>
            <span className="pagination-pages">
              {page} / {totalPages}
            </span>
            <button
              className="pagination-btn"
              onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              disabled={page === totalPages}
              title="Next page"
            >
              <ChevronRightIcon />
            </button>
            <button
              className="pagination-btn"
              onClick={() => setPage(totalPages)}
              disabled={page === totalPages}
              title="Last page"
            >
              <ChevronDoubleRightIcon />
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// Badge component for status cells
interface BadgeProps {
  variant: 'success' | 'warning' | 'danger' | 'info' | 'neutral';
  children: ReactNode;
}

export function Badge({ variant, children }: BadgeProps) {
  return <span className={`badge badge-${variant}`}>{children}</span>;
}

// Icons
function SearchIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" className="w-4 h-4">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
    </svg>
  );
}

function FilterIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" className="w-3.5 h-3.5">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z" />
    </svg>
  );
}

function CloseIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" className="w-3.5 h-3.5">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
    </svg>
  );
}

function SortIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" className="w-3 h-3 opacity-30">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M7 16V4m0 0L3 8m4-4l4 4m6 0v12m0 0l4-4m-4 4l-4-4" />
    </svg>
  );
}

function SortAscIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" className="w-3 h-3">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M5 15l7-7 7 7" />
    </svg>
  );
}

function SortDescIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" className="w-3 h-3">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
    </svg>
  );
}

function EmptyIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" className="w-8 h-8" style={{ color: 'var(--text-muted)' }}>
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
    </svg>
  );
}

function ChevronLeftIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" className="w-4 h-4">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
    </svg>
  );
}

function ChevronRightIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" className="w-4 h-4">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
    </svg>
  );
}

function ChevronDoubleLeftIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" className="w-4 h-4">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M11 19l-7-7 7-7m8 14l-7-7 7-7" />
    </svg>
  );
}

function ChevronDoubleRightIcon() {
  return (
    <svg fill="none" stroke="currentColor" viewBox="0 0 24 24" className="w-4 h-4">
      <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 5l7 7-7 7M5 5l7 7-7 7" />
    </svg>
  );
}
