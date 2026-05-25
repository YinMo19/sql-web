import { FormEvent, ReactNode, useEffect, useMemo, useState } from 'react'
import { Link, Navigate, Route, Routes, useNavigate, useParams } from 'react-router-dom'
import { Database, LogOut, Play, Table2 } from 'lucide-react'
import { api, ApiError, ColumnDetail, formatFileSize, OverviewResponse, QueryResponse, TableRows, TableStructure } from '@/lib/api'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Textarea } from '@/components/ui/textarea'

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
          <p className="text-sm text-muted-foreground">输入密码访问数据库管理页面。</p>
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
    <div className="min-h-screen bg-muted/20">
      <header className="border-b bg-background">
        <div className="flex h-14 items-center justify-between px-6">
          <Link to="/" className="flex items-center gap-2 font-semibold">
            <Database className="h-5 w-5" />
            {overview.database_stats.database_name}
          </Link>
          <div className="flex items-center gap-2">
            <Badge className="bg-background">{overview.database_stats.database_type}</Badge>
            {overview.database_stats.readonly && <Badge className="border-amber-300 bg-amber-50 text-amber-800">Read only</Badge>}
            <Button variant="ghost" size="sm" onClick={logout}><LogOut className="mr-2 h-4 w-4" />Logout</Button>
          </div>
        </div>
      </header>
      <div className="grid grid-cols-[260px_1fr]">
        <aside className="min-h-[calc(100vh-3.5rem)] border-r bg-background p-4">
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
        <main className="p-6">
          <Routes>
            <Route path="/" element={<OverviewPage overview={overview} />} />
            <Route path="/query" element={<QueryPage />} />
            <Route path="/tables/:table" element={<TableRowsPage readonly={overview.database_stats.readonly} />} />
            <Route path="/tables/:table/structure" element={<TableStructurePage readonly={overview.database_stats.readonly} onChanged={load} />} />
          </Routes>
        </main>
      </div>
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
          {!readonly && <InsertRow table={name} columns={data.columns} onChanged={load} />}
        </div>
      </div>
      <DataTable columns={data.columns} rows={data.rows} />
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
          <Link className="text-sm text-muted-foreground underline" to={`/tables/${encodeURIComponent(name)}`}>Browse rows</Link>
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

function DataTable({ columns, rows }: { columns: string[]; rows: (string | null)[][] }) {
  return (
    <div className="overflow-auto rounded-md border bg-background">
      <Table>
        <TableHeader><TableRow>{columns.map((column) => <TableHead key={column}>{column}</TableHead>)}</TableRow></TableHeader>
        <TableBody>
          {rows.map((row, rowIndex) => <TableRow key={rowIndex}>{row.map((value, cellIndex) => <TableCell className="max-w-80 truncate font-mono text-xs" key={cellIndex}>{value ?? <span className="text-muted-foreground">NULL</span>}</TableCell>)}</TableRow>)}
        </TableBody>
      </Table>
    </div>
  )
}

function PageError({ message }: { message: string }) {
  return <div className="rounded-md border border-destructive/30 bg-destructive/10 p-4 text-sm text-destructive">{message}</div>
}
