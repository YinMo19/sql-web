import { FormEvent, Fragment, ReactNode, useEffect, useMemo, useState } from 'react'
import { Link, Navigate, Route, Routes, useLocation, useNavigate, useParams } from 'react-router-dom'
import { Database, LogOut, Play, Table2 } from 'lucide-react'
import { api, ApiError, ColumnDetail, formatFileSize, OverviewResponse, QueryResponse, TableRows, TableStructure } from '@/lib/api'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Textarea } from '@/components/ui/textarea'

const REPOSITORY_URL = 'https://github.com/YinMo19/sql-web'

export default function App() {
  const [authenticated, setAuthenticated] = useState<boolean | null>(null)

  useEffect(() => {
    api.session().then((session) => setAuthenticated(session.authenticated)).catch(() => setAuthenticated(false))
  }, [])

  if (authenticated == null) {
    return <div className="flex min-h-screen items-center justify-center text-sm text-muted-foreground">Loading...</div>
  }

  return (
    <Routes>
      <Route path="/login" element={<LoginPage onLogin={() => setAuthenticated(true)} />} />
      <Route
        path="/*"
        element={authenticated ? <Shell onLogout={() => setAuthenticated(false)} /> : <Navigate to="/login" replace />}
      />
    </Routes>
  )
}

function LoginPage({ onLogin }: { onLogin: () => void }) {
  const navigate = useNavigate()
  const [password, setPassword] = useState('')
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  async function submit(event: FormEvent) {
    event.preventDefault()
    setLoading(true)
    setError('')
    try {
      await api.login(password)
      onLogin()
      navigate('/')
    } catch (error) {
      setError(error instanceof Error ? error.message : 'Login failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-muted/30 px-4">
      <Card className="w-full max-w-sm">
        <CardHeader>
          <CardTitle>SQL Web</CardTitle>
          <p className="text-sm text-muted-foreground">输入本次命令行输出的一次性密码。</p>
        </CardHeader>
        <CardContent>
          <form className="space-y-4" onSubmit={submit}>
            <Input type="password" value={password} onChange={(event) => setPassword(event.target.value)} placeholder="Password" autoFocus />
            {error && <p className="text-sm text-destructive">{error}</p>}
            <Button className="w-full" disabled={loading}>{loading ? 'Logging in...' : 'Login'}</Button>
          </form>
        </CardContent>
      </Card>
    </div>
  )
}

function Shell({ onLogout }: { onLogout: () => void }) {
  const [overview, setOverview] = useState<OverviewResponse | null>(null)
  const [error, setError] = useState('')
  const navigate = useNavigate()

  async function load() {
    try {
      setOverview(await api.overview())
      setError('')
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        navigate('/login')
        return
      }
      setError(error instanceof Error ? error.message : 'Failed to load database')
    }
  }

  useEffect(() => {
    void load()
  }, [])

  async function logout() {
    await api.logout()
    onLogout()
    navigate('/login')
  }

  if (error) return <PageError message={error} />
  if (!overview) return <div className="p-8 text-sm text-muted-foreground">Loading database...</div>

  return (
    <div className="flex min-h-screen flex-col bg-muted/20">
      <header className="border-b bg-background">
        <div className="flex h-14 items-center justify-between px-6">
          <Link to="/" className="flex items-center gap-3 font-semibold">
            <Database className="h-5 w-5" />
            <span className="flex flex-col leading-tight">
              <span>SQL-WEB by YinMo19</span>
              <span className="text-xs font-normal text-muted-foreground">{overview.database_stats.database_name}</span>
            </span>
          </Link>
          <div className="flex items-center gap-2">
            <Badge className="bg-background">{overview.database_stats.database_type}</Badge>
            {overview.database_stats.readonly && <Badge className="border-amber-300 bg-amber-50 text-amber-800">Read only</Badge>}
            <Button variant="ghost" size="sm" onClick={logout}><LogOut className="mr-2 h-4 w-4" />Logout</Button>
          </div>
        </div>
      </header>
      <div className="grid min-w-0 flex-1 grid-cols-[260px_minmax(0,1fr)]">
        <aside className="border-r bg-background p-4">
          <nav className="space-y-1">
            <NavLink to="/">Overview</NavLink>
            <NavLink to="/query">Query</NavLink>
          </nav>
          <div className="mt-6">
            <div className="mb-2 flex items-center gap-2 text-xs font-semibold uppercase text-muted-foreground">
              <Table2 className="h-3.5 w-3.5" /> Tables
            </div>
            <nav className="space-y-1">
              {overview.tables.map((table) => (
                <NavLink key={table} to={`/tables/${encodeURIComponent(table)}`}>{table}</NavLink>
              ))}
            </nav>
          </div>
        </aside>
        <main className="min-w-0 overflow-hidden p-6">
          <Routes>
            <Route path="/" element={<OverviewPage overview={overview} />} />
            <Route path="/query" element={<QueryPage />} />
            <Route path="/tables/:table" element={<TableRowsPage readonly={overview.database_stats.readonly} />} />
            <Route path="/tables/:table/preview" element={<RowPreviewPage />} />
            <Route path="/tables/:table/sql" element={<TableSqlPage />} />
            <Route path="/tables/:table/structure" element={<TableStructurePage readonly={overview.database_stats.readonly} onChanged={load} />} />
          </Routes>
        </main>
      </div>
      <footer className="border-t bg-background px-6 py-3 text-center text-xs text-muted-foreground">
        <span>By YinMo19</span>
        <a
          href={REPOSITORY_URL}
          target="_blank"
          rel="noreferrer"
          className="ml-2 inline-flex h-7 w-7 items-center justify-center rounded-full border text-foreground transition-colors hover:bg-muted"
          aria-label="GitHub repository"
        >
          <svg className="h-4 w-4" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
            <path d="M8 0C3.58 0 0 3.67 0 8.2c0 3.62 2.29 6.69 5.47 7.78.4.08.55-.18.55-.4 0-.2-.01-.85-.01-1.54-2.01.38-2.53-.5-2.69-.96-.09-.24-.48-.96-.82-1.15-.28-.15-.68-.53-.01-.54.63-.01 1.08.59 1.23.84.72 1.24 1.87.89 2.33.68.07-.53.28-.89.51-1.1-1.78-.21-3.64-.91-3.64-4.04 0-.89.31-1.62.82-2.19-.08-.21-.36-1.04.08-2.16 0 0 .67-.22 2.2.84A7.42 7.42 0 0 1 8 3.96c.68 0 1.36.09 2 .28 1.53-1.06 2.2-.84 2.2-.84.44 1.12.16 1.95.08 2.16.51.57.82 1.3.82 2.19 0 3.14-1.87 3.83-3.65 4.04.29.25.54.75.54 1.52 0 1.1-.01 1.98-.01 2.25 0 .22.15.48.55.4A8.13 8.13 0 0 0 16 8.2C16 3.67 12.42 0 8 0Z" />
          </svg>
        </a>
      </footer>
    </div>
  )
}

function NavLink({ to, children }: { to: string; children: ReactNode }) {
  return <Link to={to} className="block rounded-md px-3 py-2 text-sm hover:bg-muted">{children}</Link>
}

function OverviewPage({ overview }: { overview: OverviewResponse }) {
  const stats = overview.database_stats
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-semibold">Overview</h1>
        <p className="text-sm text-muted-foreground">SQL Web v{overview.version}</p>
      </div>
      <div className="grid gap-4 md:grid-cols-4">
        <Metric label="Database" value={stats.database_name} />
        <Metric label="Type" value={stats.database_type} />
        <Metric label="Tables" value={String(stats.table_count)} />
        <Metric label="Size" value={formatFileSize(stats.file_size)} />
      </div>
      <Card>
        <CardHeader><CardTitle>Tables</CardTitle></CardHeader>
        <CardContent>
          <div className="grid gap-2 md:grid-cols-3">
            {overview.tables.map((table) => <Link className="rounded-md border p-3 text-sm hover:bg-muted" to={`/tables/${encodeURIComponent(table)}`} key={table}>{table}</Link>)}
          </div>
        </CardContent>
      </Card>
    </div>
  )
}

function Metric({ label, value }: { label: string; value: string }) {
  return <Card><CardHeader><p className="text-sm text-muted-foreground">{label}</p><CardTitle>{value}</CardTitle></CardHeader></Card>
}

function QueryPage() {
  const [sql, setSql] = useState('SELECT 1;')
  const [result, setResult] = useState<QueryResponse | null>(null)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  async function run() {
    setLoading(true)
    setError('')
    try {
      setResult(await api.query(sql))
    } catch (error) {
      setError(error instanceof Error ? error.message : 'Query failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="space-y-4">
      <h1 className="text-2xl font-semibold">Query</h1>
      <Textarea value={sql} onChange={(event) => setSql(event.target.value)} className="min-h-40 font-mono" />
      {error && <p className="text-sm text-destructive">{error}</p>}
      <Button onClick={run} disabled={loading}><Play className="mr-2 h-4 w-4" />{loading ? 'Running...' : 'Execute'}</Button>
      {result && <ResultTable result={result} />}
    </div>
  )
}

function TableSqlPage() {
  const { table = '' } = useParams()
  const name = decodeURIComponent(table)
  const defaultSql = useMemo(() => `SELECT * FROM ${quoteSqlIdentifier(name)} LIMIT 100;`, [name])
  const [sql, setSql] = useState(defaultSql)
  const [result, setResult] = useState<QueryResponse | null>(null)
  const [error, setError] = useState('')
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    setSql(defaultSql)
    setResult(null)
    setError('')
  }, [defaultSql])

  async function run() {
    setLoading(true)
    setError('')
    try {
      setResult(await api.tableQuery(name, sql))
    } catch (error) {
      setError(error instanceof Error ? error.message : 'Query failed')
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-semibold">{name} custom SQL</h1>
        <Link className="text-sm text-muted-foreground underline" to={`/tables/${encodeURIComponent(name)}`}>Browse rows</Link>
      </div>
      <Textarea value={sql} onChange={(event) => setSql(event.target.value)} className="min-h-40 font-mono" />
      {error && <p className="text-sm text-destructive">{error}</p>}
      <Button onClick={run} disabled={loading}><Play className="mr-2 h-4 w-4" />{loading ? 'Running...' : 'Execute for table'}</Button>
      {result && <ResultTable result={result} />}
    </div>
  )
}

function TableRowsPage({ readonly }: { readonly: boolean }) {
  const { table = '' } = useParams()
  const name = decodeURIComponent(table)
  const [data, setData] = useState<TableRows | null>(null)
  const [error, setError] = useState('')
  const [page, setPage] = useState(1)

  async function load() {
    try {
      setData(await api.tableRows(name, page))
      setError('')
    } catch (error) {
      setError(error instanceof Error ? error.message : 'Failed to load rows')
    }
  }

  useEffect(() => {
    void load()
  }, [name, page])

  if (error) return <PageError message={error} />
  if (!data) return <p className="text-sm text-muted-foreground">Loading rows...</p>

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">{name}</h1>
          <p className="text-sm text-muted-foreground">{data.total_rows} rows</p>
        </div>
        <div className="flex gap-2">
          <Link to={`/tables/${encodeURIComponent(name)}/structure`}><Button variant="outline">Structure</Button></Link>
          <Link to={`/tables/${encodeURIComponent(name)}/sql`}><Button variant="outline">Custom SQL</Button></Link>
          {!readonly && <InsertRow table={name} columns={data.columns} onChanged={load} />}
        </div>
      </div>
      <DataTable columns={data.columns} rows={data.rows} previewable editable={!readonly} tableName={name} onChanged={load} />
      <div className="flex items-center justify-between">
        <Button variant="outline" disabled={page <= 1} onClick={() => setPage((page) => page - 1)}>Previous</Button>
        <span className="text-sm text-muted-foreground">Page {data.page} of {data.total_pages}</span>
        <Button variant="outline" disabled={page >= data.total_pages} onClick={() => setPage((page) => page + 1)}>Next</Button>
      </div>
    </div>
  )
}

function InsertRow({ table, columns, onChanged }: { table: string; columns: string[]; onChanged: () => void }) {
  const [open, setOpen] = useState(false)
  const [values, setValues] = useState<Record<string, string>>({})
  const [error, setError] = useState('')

  async function submit() {
    try {
      const data = Object.fromEntries(columns.map((column) => [column, values[column] || null]))
      await api.insertRow(table, data)
      setOpen(false)
      setValues({})
      onChanged()
    } catch (error) {
      setError(error instanceof Error ? error.message : 'Insert failed')
    }
  }

  if (!open) return <Button onClick={() => setOpen(true)}>Insert row</Button>

  return (
    <Card className="absolute right-6 top-24 z-10 w-96 bg-background">
      <CardHeader><CardTitle>Insert row</CardTitle></CardHeader>
      <CardContent className="space-y-3">
        {columns.map((column) => <Input key={column} placeholder={column} value={values[column] || ''} onChange={(event) => setValues({ ...values, [column]: event.target.value })} />)}
        {error && <p className="text-sm text-destructive">{error}</p>}
        <div className="flex gap-2"><Button onClick={submit}>Save</Button><Button variant="outline" onClick={() => setOpen(false)}>Cancel</Button></div>
      </CardContent>
    </Card>
  )
}

function TableStructurePage({ readonly, onChanged }: { readonly: boolean; onChanged: () => void }) {
  const { table = '' } = useParams()
  const name = decodeURIComponent(table)
  const [structure, setStructure] = useState<TableStructure | null>(null)
  const [error, setError] = useState('')

  async function load() {
    try {
      setStructure(await api.tableStructure(name))
      setError('')
    } catch (error) {
      setError(error instanceof Error ? error.message : 'Failed to load structure')
    }
  }

  useEffect(() => {
    void load()
  }, [name])

  if (error) return <PageError message={error} />
  if (!structure) return <p className="text-sm text-muted-foreground">Loading structure...</p>

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold">{name} structure</h1>
          <div className="flex gap-3 text-sm text-muted-foreground">
            <Link className="underline" to={`/tables/${encodeURIComponent(name)}`}>Browse rows</Link>
            <Link className="underline" to={`/tables/${encodeURIComponent(name)}/sql`}>Custom SQL</Link>
          </div>
        </div>
        {!readonly && <AddColumn table={name} onChanged={() => { void load(); onChanged() }} />}
      </div>
      <Card>
        <CardHeader><CardTitle>Columns</CardTitle></CardHeader>
        <CardContent><ColumnsTable table={name} columns={structure.columns} readonly={readonly} onChanged={load} /></CardContent>
      </Card>
      <Card>
        <CardHeader><CardTitle>Indexes</CardTitle></CardHeader>
        <CardContent><IndexesTable table={name} indexes={structure.indexes} columns={structure.columns.map((c) => c.name)} readonly={readonly} onChanged={load} /></CardContent>
      </Card>
    </div>
  )
}

function ColumnsTable({ table, columns, readonly, onChanged }: { table: string; columns: ColumnDetail[]; readonly: boolean; onChanged: () => void }) {
  return (
    <Table>
      <TableHeader><TableRow><TableHead>Name</TableHead><TableHead>Type</TableHead><TableHead>Nullable</TableHead><TableHead>Default</TableHead><TableHead>Key</TableHead>{!readonly && <TableHead>Actions</TableHead>}</TableRow></TableHeader>
      <TableBody>
        {columns.map((column) => (
          <TableRow key={column.name}>
            <TableCell className="font-medium">{column.name}</TableCell>
            <TableCell><code>{column.data_type}</code></TableCell>
            <TableCell>{column.nullable ? 'YES' : 'NO'}</TableCell>
            <TableCell>{column.default_value || '-'}</TableCell>
            <TableCell>{column.is_primary_key ? <Badge>PRIMARY</Badge> : null}</TableCell>
            {!readonly && <TableCell><ColumnActions table={table} column={column.name} onChanged={onChanged} /></TableCell>}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  )
}

function ColumnActions({ table, column, onChanged }: { table: string; column: string; onChanged: () => void }) {
  const [newName, setNewName] = useState(column)
  async function rename() {
    await api.renameColumn(table, column, newName)
    onChanged()
  }
  async function drop() {
    if (!confirm(`Drop column ${column}?`)) return
    await api.dropColumn(table, column)
    onChanged()
  }
  return <div className="flex gap-2"><Input className="h-8 w-32" value={newName} onChange={(event) => setNewName(event.target.value)} /><Button size="sm" variant="outline" onClick={rename}>Rename</Button><Button size="sm" variant="destructive" onClick={drop}>Drop</Button></div>
}

function AddColumn({ table, onChanged }: { table: string; onChanged: () => void }) {
  const [open, setOpen] = useState(false)
  const [name, setName] = useState('')
  const [dataType, setDataType] = useState('TEXT')
  const [nullable, setNullable] = useState(true)
  async function submit() {
    await api.addColumn(table, { name, data_type: dataType, nullable, default_value: null, primary_key: false, auto_increment: false })
    setOpen(false)
    setName('')
    onChanged()
  }
  if (!open) return <Button onClick={() => setOpen(true)}>Add column</Button>
  return <div className="flex gap-2"><Input placeholder="name" value={name} onChange={(e) => setName(e.target.value)} /><Input placeholder="type" value={dataType} onChange={(e) => setDataType(e.target.value)} /><Button variant="outline" onClick={() => setNullable(!nullable)}>{nullable ? 'Nullable' : 'Not null'}</Button><Button onClick={submit}>Save</Button><Button variant="outline" onClick={() => setOpen(false)}>Cancel</Button></div>
}

function IndexesTable({ table, indexes, columns, readonly, onChanged }: { table: string; indexes: { name: string; columns: string[]; unique: boolean; index_type: string }[]; columns: string[]; readonly: boolean; onChanged: () => void }) {
  const [name, setName] = useState('')
  const [selected, setSelected] = useState('')
  const [unique, setUnique] = useState(false)
  async function add() {
    await api.addIndex(table, { name, columns: [selected], unique })
    setName('')
    setSelected('')
    onChanged()
  }
  return (
    <div className="space-y-4">
      {!readonly && <div className="flex gap-2"><Input placeholder="index name" value={name} onChange={(e) => setName(e.target.value)} /><select className="rounded-md border px-3 text-sm" value={selected} onChange={(e) => setSelected(e.target.value)}><option value="">Column</option>{columns.map((c) => <option key={c}>{c}</option>)}</select><Button variant="outline" onClick={() => setUnique(!unique)}>{unique ? 'Unique' : 'Not unique'}</Button><Button onClick={add} disabled={!name || !selected}>Add index</Button></div>}
      <Table>
        <TableHeader><TableRow><TableHead>Name</TableHead><TableHead>Columns</TableHead><TableHead>Unique</TableHead><TableHead>Type</TableHead>{!readonly && <TableHead>Actions</TableHead>}</TableRow></TableHeader>
        <TableBody>{indexes.map((index) => <TableRow key={index.name}><TableCell>{index.name}</TableCell><TableCell>{index.columns.join(', ') || '-'}</TableCell><TableCell>{index.unique ? 'YES' : 'NO'}</TableCell><TableCell>{index.index_type}</TableCell>{!readonly && <TableCell><Button size="sm" variant="destructive" onClick={async () => { await api.dropIndex(table, index.name); onChanged() }}>Drop</Button></TableCell>}</TableRow>)}</TableBody>
      </Table>
    </div>
  )
}

function ResultTable({ result }: { result: QueryResponse }) {
  if (result.columns.length === 0) return <p className="text-sm text-muted-foreground">Rows affected: {result.rows_affected ?? 0}</p>
  return <DataTable columns={result.columns} rows={result.rows} />
}

function DataTable({
  columns,
  rows,
  editable = false,
  previewable = false,
  tableName,
  onChanged,
}: {
  columns: string[]
  rows: (string | null)[][]
  editable?: boolean
  previewable?: boolean
  tableName?: string
  onChanged?: () => void
}) {
  const [editingIndex, setEditingIndex] = useState<number | null>(null)
  const navigate = useNavigate()
  const hasActions = editable || previewable

  return (
    <div className="w-full max-w-full overflow-x-auto rounded-md border bg-background">
      <Table className="w-full table-fixed">
        <TableHeader>
          <TableRow>
            {columns.map((column) => <TableHead className="w-48 truncate" key={column}>{column}</TableHead>)}
            {hasActions && <TableHead className="w-40">Actions</TableHead>}
          </TableRow>
        </TableHeader>
        <TableBody>
          {rows.map((row, rowIndex) => (
            <Fragment key={rowIndex}>
              {editingIndex === rowIndex && tableName && onChanged && (
                <EditableRow
                  tableName={tableName}
                  columns={columns}
                  row={row}
                  onCancel={() => setEditingIndex(null)}
                  onSaved={() => {
                    setEditingIndex(null)
                    onChanged()
                  }}
                />
              )}
              <TableRow>
                {row.map((value, cellIndex) => (
                  <TableCell className="w-48 max-w-48 truncate font-mono text-xs" title={value ?? 'NULL'} key={cellIndex}>
                    {value ?? <span className="text-muted-foreground">NULL</span>}
                  </TableCell>
                ))}
                {hasActions && (
                  <TableCell className="w-40">
                    <div className="flex gap-2">
                      {previewable && tableName && (
                        <Button
                          size="sm"
                          variant="outline"
                          onClick={() => navigate(`/tables/${encodeURIComponent(tableName)}/preview`, { state: { columns, row } })}
                        >
                          Preview
                        </Button>
                      )}
                      {editable && (
                        <Button size="sm" variant="outline" onClick={() => setEditingIndex(rowIndex)}>
                          Edit
                        </Button>
                      )}
                    </div>
                  </TableCell>
                )}
              </TableRow>
            </Fragment>
          ))}
        </TableBody>
      </Table>
    </div>
  )
}

function RowPreviewPage() {
  const { table = '' } = useParams()
  const name = decodeURIComponent(table)
  const location = useLocation()
  const state = location.state as { columns?: string[]; row?: (string | null)[] } | null
  const columns = state?.columns
  const row = state?.row

  if (!columns || !row) {
    return <PageError message="No row preview data is available. Please open preview from the table rows page." />
  }

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-semibold">{name} row preview</h1>
        <Link className="text-sm text-muted-foreground underline" to={`/tables/${encodeURIComponent(name)}`}>Back to rows</Link>
      </div>
      <div className="grid gap-3 md:grid-cols-2 xl:grid-cols-3">
        {columns.map((column, index) => {
          const value = formatPreviewValue(row[index])
          return (
            <Card key={column} className="min-w-0">
              <CardHeader>
                <CardTitle className="text-sm">{column}</CardTitle>
              </CardHeader>
              <CardContent>
                {row[index] === null ? (
                  <code className="text-xs text-muted-foreground">NULL</code>
                ) : (
                  <pre className="max-h-[60vh] overflow-auto whitespace-pre-wrap break-words text-xs leading-relaxed">{value}</pre>
                )}
              </CardContent>
            </Card>
          )
        })}
      </div>
    </div>
  )
}

function EditableRow({
  tableName,
  columns,
  row,
  onCancel,
  onSaved,
}: {
  tableName: string
  columns: string[]
  row: (string | null)[]
  onCancel: () => void
  onSaved: () => void
}) {
  const [values, setValues] = useState<Record<string, string>>(() => Object.fromEntries(columns.map((column, index) => [column, row[index] ?? ''])))
  const [error, setError] = useState('')
  const [saving, setSaving] = useState(false)

  async function save() {
    const where_clause = Object.fromEntries(
      columns
        .map((column, index) => [column, row[index]] as const)
        .filter((entry): entry is readonly [string, string] => entry[1] !== null),
    )

    if (Object.keys(where_clause).length === 0) {
      setError('Cannot update a row where every original value is NULL.')
      return
    }

    setSaving(true)
    setError('')
    try {
      const data = Object.fromEntries(columns.map((column) => [column, values[column] || null]))
      await api.updateRow(tableName, data, where_clause)
      onSaved()
    } catch (error) {
      setError(error instanceof Error ? error.message : 'Update failed')
    } finally {
      setSaving(false)
    }
  }

  return (
    <TableRow className="bg-muted/50 hover:bg-muted/50">
      <TableCell colSpan={columns.length + 1}>
        <div className="space-y-3 p-2">
          <div className="grid gap-2 md:grid-cols-2 xl:grid-cols-3">
            {columns.map((column) => (
              <label key={column} className="space-y-1 text-xs font-medium text-muted-foreground">
                <span>{column}</span>
                <Input value={values[column] ?? ''} onChange={(event) => setValues({ ...values, [column]: event.target.value })} />
              </label>
            ))}
          </div>
          {error && <p className="text-sm text-destructive">{error}</p>}
          <div className="flex gap-2">
            <Button size="sm" onClick={save} disabled={saving}>{saving ? 'Saving...' : 'Save changes'}</Button>
            <Button size="sm" variant="outline" onClick={onCancel}>Cancel</Button>
          </div>
        </div>
      </TableCell>
    </TableRow>
  )
}

function PageError({ message }: { message: string }) {
  return <div className="rounded-md border border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive">{message}</div>
}

function formatPreviewValue(value: string | null) {
  if (value === null) return 'NULL'

  try {
    return JSON.stringify(JSON.parse(value), null, 2)
  } catch {
    return value
  }
}

function quoteSqlIdentifier(identifier: string) {
  return `"${identifier.replace(/"/g, '""')}"`
}
