export type DatabaseStats = {
  database_name: string
  database_type: string
  file_size: number | null
  table_count: number
  index_count: number
  trigger_count: number
  view_count: number
  created: string | null
  modified: string | null
  readonly: boolean
}

export type OverviewResponse = {
  database_stats: DatabaseStats
  tables: string[]
  version: string
}

export type QueryResponse = {
  columns: string[]
  rows: (string | null)[][]
  total_rows: number | null
  page: number
  per_page: number
  total_pages: number
  error: string | null
  rows_affected: number | null
}

export type TableRows = {
  name: string
  columns: string[]
  rows: (string | null)[][]
  total_rows: number
  page: number
  per_page: number
  total_pages: number
}

export type ColumnDetail = {
  name: string
  data_type: string
  nullable: boolean
  default_value: string | null
  is_primary_key: boolean
  is_auto_increment: boolean
  max_length: number | null
}

export type IndexDetail = {
  name: string
  columns: string[]
  unique: boolean
  index_type: string
}

export type TableStructure = {
  name: string
  columns: ColumnDetail[]
  indexes: IndexDetail[]
  foreign_keys: unknown[]
  triggers: unknown[]
  create_sql: string | null
}

export type CreateColumnRequest = {
  name: string
  data_type: string
  nullable: boolean
  default_value: string | null
  primary_key: boolean
  auto_increment: boolean
}

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message)
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    credentials: 'same-origin',
    headers: init?.body ? { 'content-type': 'application/json', ...init.headers } : init?.headers,
    ...init,
  })

  if (!response.ok) {
    let message = response.statusText
    try {
      const body = (await response.json()) as { error?: string }
      message = body.error || message
    } catch {
      // ignore non-json error bodies
    }
    throw new ApiError(response.status, message)
  }

  return (await response.json()) as T
}

export const api = {
  login: (password: string) =>
    request<{ authenticated: boolean }>('/api/login', {
      method: 'POST',
      body: JSON.stringify({ password }),
    }),
  logout: () => request<{ authenticated: boolean }>('/api/logout', { method: 'POST' }),
  session: () => request<{ authenticated: boolean }>('/api/session'),
  overview: () => request<OverviewResponse>('/api/overview'),
  tableRows: (table: string, page = 1, perPage?: number) => {
    const params = new URLSearchParams({ page: String(page) })
    if (perPage) params.set('per_page', String(perPage))
    return request<TableRows>(`/api/tables/${encodeURIComponent(table)}/rows?${params}`)
  },
  tableStructure: (table: string) => request<TableStructure>(`/api/tables/${encodeURIComponent(table)}/structure`),
  query: (sql: string) =>
    request<QueryResponse>('/api/query', {
      method: 'POST',
      body: JSON.stringify({ sql }),
    }),
  insertRow: (table: string, data: Record<string, string | null>) =>
    request<QueryResponse>(`/api/tables/${encodeURIComponent(table)}/rows`, {
      method: 'POST',
      body: JSON.stringify({ data }),
    }),
  updateRow: (
    table: string,
    data: Record<string, string | null>,
    where_clause: Record<string, string>,
  ) =>
    request<QueryResponse>(`/api/tables/${encodeURIComponent(table)}/rows`, {
      method: 'PATCH',
      body: JSON.stringify({ data, where_clause }),
    }),
  addColumn: (table: string, body: CreateColumnRequest) =>
    request<QueryResponse>(`/api/tables/${encodeURIComponent(table)}/columns`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  renameColumn: (table: string, column: string, newName: string) =>
    request<QueryResponse>(
      `/api/tables/${encodeURIComponent(table)}/columns/${encodeURIComponent(column)}`,
      {
        method: 'PATCH',
        body: JSON.stringify({ new_name: newName }),
      },
    ),
  dropColumn: (table: string, column: string) =>
    request<QueryResponse>(
      `/api/tables/${encodeURIComponent(table)}/columns/${encodeURIComponent(column)}`,
      { method: 'DELETE' },
    ),
  addIndex: (table: string, body: { name: string; columns: string[]; unique: boolean }) =>
    request<QueryResponse>(`/api/tables/${encodeURIComponent(table)}/indexes`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),
  dropIndex: (table: string, index: string) =>
    request<QueryResponse>(
      `/api/tables/${encodeURIComponent(table)}/indexes/${encodeURIComponent(index)}`,
      { method: 'DELETE' },
    ),
}

export function formatFileSize(size: number | null) {
  if (size == null) return 'Unknown'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  let value = size
  let unit = 0
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024
    unit += 1
  }
  return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`
}
