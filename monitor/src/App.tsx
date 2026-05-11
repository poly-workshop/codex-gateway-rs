import { useCallback, useEffect, useMemo, useState } from "react"
import {
  Activity01Icon,
  AlertCircleIcon,
  ApiGatewayIcon,
  ArrowReloadHorizontalIcon,
  CheckmarkCircle01Icon,
  Configuration01Icon,
  Database01Icon,
  Key01Icon,
  MessageMultiple01Icon,
  ServerStack01Icon,
  ShieldKeyIcon,
  TokenCircleIcon,
  UserGroupIcon,
} from "@hugeicons/core-free-icons"
import { HugeiconsIcon } from "@hugeicons/react"
import type { IconSvgElement } from "@hugeicons/react"

import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import {
  Card,
  CardAction,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card"
import {
  Empty,
  EmptyDescription,
  EmptyHeader,
  EmptyMedia,
  EmptyTitle,
} from "@/components/ui/empty"
import { Progress } from "@/components/ui/progress"
import { Separator } from "@/components/ui/separator"
import { Skeleton } from "@/components/ui/skeleton"
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import {
  fetchMonitorOverview,
  type CodexKeyOverview,
  type ConfigOverview,
  type MemberOverview,
  type MonitorOverview,
  type UpstreamOverview,
  type UsageEventOverview,
} from "@/lib/api"
import { cn } from "@/lib/utils"

const numberFormatter = new Intl.NumberFormat("en-US")
const compactFormatter = new Intl.NumberFormat("en-US", {
  notation: "compact",
  maximumFractionDigits: 1,
})
const decimalFormatter = new Intl.NumberFormat("en-US", {
  maximumFractionDigits: 2,
})
const percentFormatter = new Intl.NumberFormat("en-US", {
  style: "percent",
  maximumFractionDigits: 1,
})
type LoadState = {
  loading: boolean
  data: MonitorOverview | null
  error: string | null
}

type HistoryPoint = {
  generatedAt: string
  credits: number
  requests: number
  messages: number
  wsConnections: number
  healthyUpstreams: number
}

export function App() {
  const [state, setState] = useState<LoadState>({
    loading: true,
    data: null,
    error: null,
  })
  const [history, setHistory] = useState<HistoryPoint[]>([])
  const [autoRefresh, setAutoRefresh] = useState(true)
  const [refreshIntervalMs, setRefreshIntervalMs] = useState(30_000)

  const acceptOverview = useCallback((data: MonitorOverview) => {
    setState({ loading: false, data, error: null })
    setHistory((current) => {
      const point: HistoryPoint = {
        generatedAt: data.generatedAt,
        credits: data.summary.credits,
        requests: data.summary.requestCount,
        messages: data.summary.messageCount,
        wsConnections: data.summary.wsConnectionCount,
        healthyUpstreams: data.summary.healthyUpstreamKeyCount,
      }
      const next =
        current.at(-1)?.generatedAt === point.generatedAt
          ? [...current.slice(0, -1), point]
          : [...current, point]
      return next.slice(-40)
    })
  }, [])

  const fetchAndAccept = useCallback((signal?: AbortSignal) => {
    return fetchMonitorOverview(signal)
      .then(acceptOverview)
      .catch((error: unknown) => {
        if (error instanceof DOMException && error.name === "AbortError") {
          return
        }

        setState((current) => ({
          loading: false,
          data: current.data,
          error: error instanceof Error ? error.message : "Failed to load monitor data",
        }))
      })
  }, [acceptOverview])

  const load = useCallback((signal?: AbortSignal) => {
    setState((current) => ({
      loading: true,
      data: current.data,
      error: null,
    }))

    fetchAndAccept(signal)
  }, [fetchAndAccept])

  useEffect(() => {
    const controller = new AbortController()
    fetchAndAccept(controller.signal)
    return () => controller.abort()
  }, [fetchAndAccept])

  useEffect(() => {
    if (!autoRefresh) {
      return
    }

    const interval = window.setInterval(() => {
      const controller = new AbortController()
      fetchAndAccept(controller.signal)
    }, refreshIntervalMs)

    return () => window.clearInterval(interval)
  }, [autoRefresh, fetchAndAccept, refreshIntervalMs])

  const overview = state.data

  return (
    <div className="min-h-svh bg-background">
      <main className="mx-auto flex w-full max-w-7xl flex-col gap-6 px-4 py-5 sm:px-6 lg:px-8">
        <header className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
          <div className="min-w-0">
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <HugeIcon icon={ApiGatewayIcon} />
              <span>Codex Gateway Monitor</span>
            </div>
            <h1 className="mt-2 text-2xl font-medium tracking-normal sm:text-3xl">
              Usage and configuration
            </h1>
            <p className="mt-2 max-w-3xl text-sm text-muted-foreground">
              {overview
                ? `UTC ${overview.date} · generated ${formatDateTime(overview.generatedAt)}`
                : "Loading live Codex gateway state"}
            </p>
          </div>
          <div className="flex flex-wrap items-center gap-2 sm:justify-end">
            <Button
              variant="outline"
              onClick={() => load()}
              disabled={state.loading}
            >
              <HugeIcon icon={ArrowReloadHorizontalIcon} data-icon="inline-start" />
              Refresh
            </Button>
            <Button
              variant={autoRefresh ? "default" : "outline"}
              onClick={() => setAutoRefresh((value) => !value)}
            >
              Auto
            </Button>
            {[15_000, 30_000, 60_000].map((interval) => (
              <Button
                key={interval}
                variant={refreshIntervalMs === interval ? "default" : "outline"}
                onClick={() => setRefreshIntervalMs(interval)}
              >
                {formatRefreshInterval(interval)}
              </Button>
            ))}
          </div>
        </header>

        {state.error ? (
          <Alert variant="destructive">
            <HugeIcon icon={AlertCircleIcon} />
            <AlertTitle>Monitor API unavailable</AlertTitle>
            <AlertDescription>{state.error}</AlertDescription>
          </Alert>
        ) : null}

        {overview ? (
          <Dashboard overview={overview} history={history} />
        ) : (
          <DashboardSkeleton />
        )}
      </main>
    </div>
  )
}

function Dashboard({
  overview,
  history,
}: {
  overview: MonitorOverview
  history: HistoryPoint[]
}) {
  return (
    <div className="flex flex-col gap-6">
      <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        <MetricCard
          title="Codex credits"
          value={formatCredits(overview.summary.credits)}
          description={`${formatNumber(overview.summary.totalTokens)} raw tokens`}
          icon={TokenCircleIcon}
        />
        <MetricCard
          title="Requests"
          value={formatNumber(overview.summary.requestCount)}
          description={`${formatNumber(overview.summary.messageCount)} messages`}
          icon={Activity01Icon}
        />
        <MetricCard
          title="WebSocket connections"
          value={formatNumber(overview.summary.wsConnectionCount)}
          description="Recorded today"
          icon={MessageMultiple01Icon}
        />
        <MetricCard
          title="Healthy upstreams"
          value={formatNumber(overview.summary.healthyUpstreamKeyCount)}
          description={`${formatNumber(overview.summary.activeUpstreamKeyCount)} active upstream keys`}
          icon={ServerStack01Icon}
        />
      </section>

      <section className="grid gap-4 lg:grid-cols-3">
        <StatusCard
          title="Members"
          active={overview.summary.activeMemberCount}
          total={overview.summary.memberCount}
          icon={UserGroupIcon}
        />
        <StatusCard
          title="Codex keys"
          active={overview.summary.activeCodexKeyCount}
          total={overview.summary.codexKeyCount}
          icon={ShieldKeyIcon}
        />
        <StatusCard
          title="Healthy upstreams"
          active={overview.summary.healthyUpstreamKeyCount}
          total={overview.summary.upstreamKeyCount}
          icon={ServerStack01Icon}
        />
      </section>

      <MonitorCharts overview={overview} history={history} />

      <Tabs defaultValue="overview" className="gap-4">
        <TabsList className="max-w-full flex-wrap justify-start overflow-x-auto rounded-2xl">
          <TabsTrigger value="overview">Overview</TabsTrigger>
          <TabsTrigger value="members">Members</TabsTrigger>
          <TabsTrigger value="keys">Keys</TabsTrigger>
          <TabsTrigger value="upstreams">Upstreams</TabsTrigger>
          <TabsTrigger value="events">Events</TabsTrigger>
          <TabsTrigger value="config">Config</TabsTrigger>
        </TabsList>
        <TabsContent value="overview">
          <OverviewPanel overview={overview} />
        </TabsContent>
        <TabsContent value="members">
          <MembersPanel
            members={overview.members}
            limits={overview.config.limits}
          />
        </TabsContent>
        <TabsContent value="keys">
          <CodexKeysTable keys={overview.codexKeys} />
        </TabsContent>
        <TabsContent value="upstreams">
          <UpstreamsTable upstreams={overview.upstreams} />
        </TabsContent>
        <TabsContent value="events">
          <EventsTable events={overview.recentEvents} />
        </TabsContent>
        <TabsContent value="config">
          <ConfigPanel config={overview.config} />
        </TabsContent>
      </Tabs>
    </div>
  )
}

function OverviewPanel({ overview }: { overview: MonitorOverview }) {
  const topMembers = useMemo(
    () => overview.members.slice(0, 5),
    [overview.members]
  )
  const topUpstreams = useMemo(
    () => overview.upstreams.slice(0, 5),
    [overview.upstreams]
  )
  const maxMemberCredits = Math.max(
    ...topMembers.map((member) => member.credits),
    1
  )

  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle>Member usage</CardTitle>
          <CardDescription>Highest Codex credit usage today</CardDescription>
        </CardHeader>
        <CardContent>
          {topMembers.length ? (
            <div className="flex flex-col gap-4">
              {topMembers.map((member) => (
                <UsageRow
                  key={member.id}
                  label={member.name}
                  meta={`${formatNumber(member.requestCount)} requests`}
                  value={member.credits}
                  ratio={ratio(member.credits, maxMemberCredits)}
                />
              ))}
            </div>
          ) : (
            <EmptyState
              icon={UserGroupIcon}
              title="No members"
              description="Create members with the admin CLI to start tracking usage."
            />
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>Upstream capacity</CardTitle>
          <CardDescription>Current concurrency and health state</CardDescription>
        </CardHeader>
        <CardContent>
          {topUpstreams.length ? (
            <div className="flex flex-col gap-4">
              {topUpstreams.map((upstream) => (
                <CapacityRow key={upstream.id} upstream={upstream} />
              ))}
            </div>
          ) : (
            <EmptyState
              icon={ServerStack01Icon}
              title="No upstream keys"
              description="Add a legitimate upstream API key before serving traffic."
            />
          )}
        </CardContent>
      </Card>
    </div>
  )
}

function MonitorCharts({
  overview,
  history,
}: {
  overview: MonitorOverview
  history: HistoryPoint[]
}) {
  const memberCredits = useMemo(
    () =>
      overview.members
        .filter((member) => member.credits > 0)
        .sort((left, right) => right.credits - left.credits)
        .slice(0, 8)
        .map((member) => ({
          label: member.name,
          value: member.credits,
        })),
    [overview.members]
  )
  const upstreamLoad = useMemo(
    () =>
      [...overview.upstreams]
        .sort(
          (left, right) =>
            ratio(
              right.currentConcurrentRequests,
              right.maxConcurrentRequests
            ) -
            ratio(left.currentConcurrentRequests, left.maxConcurrentRequests)
        )
        .slice(0, 8)
        .map((upstream) => ({
          label: upstream.name,
          value: upstream.currentConcurrentRequests,
          max: upstream.maxConcurrentRequests,
        })),
    [overview.upstreams]
  )
  const recentErrorCounts = useMemo(() => {
    const counts = new Map<string, number>()
    for (const event of overview.recentEvents) {
      if (event.success) {
        continue
      }
      const key = event.errorClass ?? event.statusCode?.toString() ?? "error"
      counts.set(key, (counts.get(key) ?? 0) + 1)
    }
    return Array.from(counts, ([label, value]) => ({ label, value }))
      .sort((left, right) => right.value - left.value)
      .slice(0, 8)
  }, [overview.recentEvents])

  return (
    <section className="grid gap-4 xl:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)]">
      <LineChartCard
        title="Live usage"
        description={`${formatNumber(history.length)} samples retained in this browser`}
        points={history}
        series={[
          { key: "credits", label: "credits" },
          { key: "requests", label: "requests" },
          { key: "messages", label: "messages" },
        ]}
      />
      <div className="grid gap-4 md:grid-cols-2 xl:grid-cols-1">
        <BarChartCard
          title="Member credits"
          description="Top members today"
          data={memberCredits}
          emptyTitle="No member usage"
          emptyDescription="Credits will appear after requests finish."
          valueFormatter={formatCredits}
        />
        <CapacityChartCard
          title="Upstream load"
          description="Current concurrent requests"
          data={upstreamLoad}
        />
      </div>
      <BarChartCard
        title="Recent failures"
        description="Last 50 usage events"
        data={recentErrorCounts}
        emptyTitle="No recent failures"
        emptyDescription="Failed events will appear here when recorded."
        valueFormatter={formatNumber}
      />
      <QuotaPressureCard members={overview.members} />
    </section>
  )
}

function MembersPanel({
  members,
  limits,
}: {
  members: MemberOverview[]
  limits: ConfigOverview["limits"]
}) {
  const activeMembers = members.filter((member) => member.status === "active")
  const totalMemberSlots = members.reduce(
    (total, member) => total + member.maxConcurrentRequests,
    0
  )
  const currentMemberLoad = members.reduce(
    (total, member) => total + member.currentConcurrentRequests,
    0
  )

  return (
    <div className="flex flex-col gap-4">
      <section className="grid gap-4 md:grid-cols-2 xl:grid-cols-5">
        <MetricCard
          title="Default member limit"
          value={formatNumber(limits.defaultMemberConcurrency)}
          description="Concurrent requests for new members"
          icon={Configuration01Icon}
        />
        <MetricCard
          title="Default 5h quota"
          value={formatQuota(limits.defaultMember5hQuota)}
          description="Request window for new members"
          icon={Activity01Icon}
        />
        <MetricCard
          title="Default weekly quota"
          value={formatQuota(limits.defaultMemberWeeklyQuota)}
          description="7 day request window"
          icon={TokenCircleIcon}
        />
        <MetricCard
          title="Member limit slots"
          value={formatNumber(totalMemberSlots)}
          description={`${formatNumber(activeMembers.length)} active members`}
          icon={UserGroupIcon}
        />
        <LimitLoadCard
          title="Current member load"
          current={currentMemberLoad}
          max={totalMemberSlots}
          icon={Activity01Icon}
        />
      </section>

      {members.length ? (
        <section className="grid gap-4 lg:grid-cols-2 xl:grid-cols-3">
          {members.map((member) => (
            <MemberLimitCard key={member.id} member={member} />
          ))}
        </section>
      ) : (
        <EmptyState
          icon={UserGroupIcon}
          title="No members"
          description="Members will appear here after they are created."
        />
      )}

      <MembersTable members={members} />
    </div>
  )
}

function MemberLimitCard({ member }: { member: MemberOverview }) {
  const concurrencyRatio = ratio(
    member.currentConcurrentRequests,
    member.maxConcurrentRequests
  )

  return (
    <Card>
      <CardHeader>
        <CardTitle>{member.name}</CardTitle>
        <CardDescription>Created {formatDateTime(member.createdAt)}</CardDescription>
        <CardAction>
          <StatusBadge status={member.status} />
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="flex flex-col gap-2">
          <div className="flex items-center justify-between gap-3">
            <div className="text-sm text-muted-foreground">Concurrency limit</div>
            <div className="font-mono text-sm">
              {member.currentConcurrentRequests}/{member.maxConcurrentRequests}
            </div>
          </div>
          <Progress
            value={toPercentValue(concurrencyRatio)}
            aria-label={`${member.name} member concurrency`}
          />
        </div>
        <Separator />
        <WindowQuotaRow
          label="5h quota"
          used={member.fiveHourUsage.credits}
          quota={member.fiveHourQuota}
          meta={`${formatNumber(member.fiveHourUsage.requestCount)} requests`}
        />
        <WindowQuotaRow
          label="Weekly quota"
          used={member.weeklyUsage.credits}
          quota={member.weeklyQuota}
          meta={`${formatNumber(member.weeklyUsage.requestCount)} requests`}
        />
        <Separator />
        <LimitDetailRow
          label="Scheduling weight"
          value={formatDecimal(member.weight)}
        />
        <LimitDetailRow
          label="Credits"
          value={formatCredits(member.credits)}
          meta={`${formatNumber(member.totalTokens)} raw`}
        />
        <LimitDetailRow
          label="Requests"
          value={formatNumber(member.requestCount)}
          meta={`${formatNumber(member.messageCount)} messages`}
        />
      </CardContent>
    </Card>
  )
}

function MembersTable({ members }: { members: MemberOverview[] }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Member details</CardTitle>
        <CardDescription>Usage rollups and per-member gateway limits</CardDescription>
      </CardHeader>
      <CardContent>
        {members.length ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>5h quota</TableHead>
                <TableHead>Weekly quota</TableHead>
                <TableHead>Weight</TableHead>
                <TableHead>Credits</TableHead>
                <TableHead>Requests</TableHead>
                <TableHead>Messages</TableHead>
                <TableHead>Concurrency</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {members.map((member) => (
                <TableRow key={member.id}>
                  <TableCell className="font-medium">{member.name}</TableCell>
                  <TableCell>
                    <StatusBadge status={member.status} />
                  </TableCell>
                  <TableCell>
                    {formatWindowUsage(
                      member.fiveHourUsage.credits,
                      member.fiveHourQuota
                    )}
                  </TableCell>
                  <TableCell>
                    {formatWindowUsage(
                      member.weeklyUsage.credits,
                      member.weeklyQuota
                    )}
                  </TableCell>
                  <TableCell>
                    {formatDecimal(member.weight)}
                  </TableCell>
                  <TableCell>
                    <div className="flex min-w-32 flex-col gap-1">
                      <span>{formatCredits(member.credits)}</span>
                      <span className="text-xs text-muted-foreground">
                        {formatNumber(member.totalTokens)} raw
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>{formatNumber(member.requestCount)}</TableCell>
                  <TableCell>{formatNumber(member.messageCount)}</TableCell>
                  <TableCell>
                    {member.currentConcurrentRequests}/
                    {member.maxConcurrentRequests}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : (
          <EmptyState
            icon={UserGroupIcon}
            title="No members"
            description="Members will appear here after they are created."
          />
        )}
      </CardContent>
    </Card>
  )
}

function CodexKeysTable({ keys }: { keys: CodexKeyOverview[] }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Codex keys</CardTitle>
        <CardDescription>Safe key prefixes and owner status</CardDescription>
      </CardHeader>
      <CardContent>
        {keys.length ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Prefix</TableHead>
                <TableHead>Member</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Created</TableHead>
                <TableHead>Last used</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {keys.map((key) => (
                <TableRow key={key.id}>
                  <TableCell className="font-mono text-xs">{key.prefix}</TableCell>
                  <TableCell>{key.memberName}</TableCell>
                  <TableCell>
                    <StatusBadge status={key.status} />
                  </TableCell>
                  <TableCell>{formatDateTime(key.createdAt)}</TableCell>
                  <TableCell>{formatOptionalDateTime(key.lastUsedAt)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : (
          <EmptyState
            icon={Key01Icon}
            title="No Codex keys"
            description="Codex key prefixes will appear here after creation."
          />
        )}
      </CardContent>
    </Card>
  )
}

function UpstreamsTable({ upstreams }: { upstreams: UpstreamOverview[] }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Upstreams</CardTitle>
        <CardDescription>Capacity, protocol support, and cooldown state</CardDescription>
      </CardHeader>
      <CardContent>
        {upstreams.length ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Health</TableHead>
                <TableHead>Protocols</TableHead>
                <TableHead>Concurrency</TableHead>
                <TableHead>Failures</TableHead>
                <TableHead>Cooldown</TableHead>
                <TableHead>Last used</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {upstreams.map((upstream) => (
                <TableRow key={upstream.id}>
                  <TableCell>
                    <div className="flex min-w-40 flex-col gap-1">
                      <span className="font-medium">{upstream.name}</span>
                      <span className="text-xs text-muted-foreground">
                        weight {formatDecimal(upstream.weight)}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <StatusBadge
                      status={upstream.healthy ? "healthy" : upstream.status}
                    />
                  </TableCell>
                  <TableCell>
                    <div className="flex gap-1">
                      {upstream.supportsHttp ? (
                        <Badge variant="secondary">HTTP</Badge>
                      ) : null}
                      {upstream.supportsWs ? (
                        <Badge variant="secondary">WS</Badge>
                      ) : null}
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex min-w-36 flex-col gap-2">
                      <span>
                        {upstream.currentConcurrentRequests}/
                        {upstream.maxConcurrentRequests}
                      </span>
                      <Progress
                        value={toPercentValue(
                          ratio(
                            upstream.currentConcurrentRequests,
                            upstream.maxConcurrentRequests
                          )
                        )}
                        aria-label={`${upstream.name} concurrency`}
                      />
                    </div>
                  </TableCell>
                  <TableCell>{formatNumber(upstream.failureCount)}</TableCell>
                  <TableCell>{formatOptionalDateTime(upstream.cooldownUntil)}</TableCell>
                  <TableCell>{formatOptionalDateTime(upstream.lastUsedAt)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : (
          <EmptyState
            icon={ServerStack01Icon}
            title="No upstream keys"
            description="Upstream status will appear after keys are configured."
          />
        )}
      </CardContent>
    </Card>
  )
}

function EventsTable({ events }: { events: UsageEventOverview[] }) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Recent events</CardTitle>
        <CardDescription>Last 50 recorded usage events</CardDescription>
      </CardHeader>
      <CardContent>
        {events.length ? (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Time</TableHead>
                <TableHead>Member</TableHead>
                <TableHead>Route</TableHead>
                <TableHead>Model</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Credits</TableHead>
                <TableHead>Duration</TableHead>
                <TableHead>Error</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {events.map((event) => (
                <TableRow key={event.id}>
                  <TableCell>{formatDateTime(event.createdAt)}</TableCell>
                  <TableCell>
                    <div className="flex min-w-36 flex-col gap-1">
                      <span>{event.memberName}</span>
                      <span className="font-mono text-xs text-muted-foreground">
                        {event.codexKeyPrefix}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="flex min-w-44 flex-col gap-1">
                      <span className="font-mono text-xs">{event.path}</span>
                      <span className="text-xs text-muted-foreground">
                        {event.protocol}
                        {event.upstreamName ? ` · ${event.upstreamName}` : ""}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell>{event.model ?? "unknown"}</TableCell>
                  <TableCell>
                    <StatusBadge
                      status={
                        event.success
                          ? "success"
                          : event.statusCode
                            ? `${event.statusCode}`
                            : "error"
                      }
                    />
                  </TableCell>
                  <TableCell>{formatCredits(event.credits)}</TableCell>
                  <TableCell>{formatDuration(event.durationMs)}</TableCell>
                  <TableCell>{event.errorClass ?? "none"}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        ) : (
          <EmptyState
            icon={Database01Icon}
            title="No usage events"
            description="Codex proxy traffic will populate this table after requests finish."
          />
        )}
      </CardContent>
    </Card>
  )
}

function ConfigPanel({ config }: { config: ConfigOverview }) {
  return (
    <div className="grid gap-4 lg:grid-cols-2">
      <ConfigCard
        title="Server"
        icon={ApiGatewayIcon}
        rows={[["Bind address", config.server.bindAddr]]}
      />
      <ConfigCard
        title="Upstream"
        icon={ServerStack01Icon}
        rows={[
          ["HTTP base URL", config.upstream.httpBaseUrl],
          ["WS base URL", config.upstream.wsBaseUrl],
          ["Timeout", `${config.upstream.timeoutSecs}s`],
          ["Retry attempts", formatNumber(config.upstream.retryAttempts)],
          ["Cooldown", `${config.upstream.cooldownSecs}s`],
          [
            "Failure threshold",
            formatNumber(config.upstream.maxFailuresBeforeCooldown),
          ],
        ]}
      />
      <ConfigCard
        title="Credit accounting"
        icon={TokenCircleIcon}
        rows={[
          ["Mode", config.credit.accounting],
          [
            "Unknown model credits",
            formatCredits(config.credit.unknownModelMessageCredits),
          ],
          [
            "Unknown usage credits",
            formatCredits(config.credit.unknownUsageCredits),
          ],
        ]}
      />
      <ConfigCard
        title="Limits"
        icon={Configuration01Icon}
        rows={[
          [
            "Default member concurrency",
            formatNumber(config.limits.defaultMemberConcurrency),
          ],
          [
            "Default member 5h quota",
            formatQuota(config.limits.defaultMember5hQuota),
          ],
          [
            "Default member weekly quota",
            formatQuota(config.limits.defaultMemberWeeklyQuota),
          ],
          [
            "Default upstream concurrency",
            formatNumber(config.limits.defaultUpstreamConcurrency),
          ],
          ["WS idle timeout", `${config.limits.wsIdleTimeoutSecs}s`],
          [
            "WS upstream ping",
            config.limits.wsUpstreamPingIntervalSecs
              ? `${config.limits.wsUpstreamPingIntervalSecs}s`
              : "disabled",
          ],
          ["WS max connection", `${config.limits.wsMaxConnectionSecs}s`],
          [
            "WS max messages",
            formatNumber(config.limits.wsMaxMessagesPerConnection),
          ],
        ]}
      />
    </div>
  )
}

function ConfigCard({
  title,
  icon,
  rows,
}: {
  title: string
  icon: IconSvgElement
  rows: Array<[string, string]>
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardAction>
          <IconBox icon={icon} />
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {rows.map(([label, value], index) => (
          <div key={label} className="flex flex-col gap-3">
            {index ? <Separator /> : null}
            <div className="grid gap-1 sm:grid-cols-[minmax(0,0.8fr)_minmax(0,1fr)] sm:items-center">
              <div className="text-sm text-muted-foreground">{label}</div>
              <div className="min-w-0 break-words font-mono text-sm">{value}</div>
            </div>
          </div>
        ))}
      </CardContent>
    </Card>
  )
}

function MetricCard({
  title,
  value,
  description,
  icon,
}: {
  title: string
  value: string
  description: string
  icon: IconSvgElement
}) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
        <CardAction>
          <IconBox icon={icon} />
        </CardAction>
      </CardHeader>
      <CardContent>
        <div className="text-3xl font-medium">{value}</div>
      </CardContent>
    </Card>
  )
}

type ChartSeries = {
  key: keyof Omit<HistoryPoint, "generatedAt">
  label: string
}

function LineChartCard({
  title,
  description,
  points,
  series,
}: {
  title: string
  description: string
  points: HistoryPoint[]
  series: ChartSeries[]
}) {
  const chartPoints = points.length ? points : []
  const maxValue = Math.max(
    ...chartPoints.flatMap((point) =>
      series.map((item) => Number(point[item.key]) || 0)
    ),
    1
  )

  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-4">
        <div className="h-72 min-w-0">
          {chartPoints.length > 1 ? (
            <svg
              viewBox="0 0 720 260"
              role="img"
              aria-label={`${title} chart`}
              className="size-full overflow-visible"
              preserveAspectRatio="none"
            >
              <ChartGrid />
              {series.map((item, index) => (
                <polyline
                  key={item.key}
                  fill="none"
                  stroke={chartStroke(index)}
                  strokeWidth="3"
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  points={polylinePoints(chartPoints, item.key, maxValue)}
                />
              ))}
            </svg>
          ) : (
            <EmptyState
              icon={Activity01Icon}
              title="Collecting samples"
              description="Auto refresh will populate this chart."
            />
          )}
        </div>
        <div className="flex flex-wrap gap-2">
          {series.map((item, index) => (
            <Badge key={item.key} variant="secondary">
              <span
                className="size-2 rounded-full"
                style={{ backgroundColor: chartStroke(index) }}
              />
              {item.label}
            </Badge>
          ))}
        </div>
      </CardContent>
    </Card>
  )
}

function BarChartCard({
  title,
  description,
  data,
  emptyTitle,
  emptyDescription,
  valueFormatter,
}: {
  title: string
  description: string
  data: Array<{ label: string; value: number }>
  emptyTitle: string
  emptyDescription: string
  valueFormatter: (value: number) => string
}) {
  const maxValue = Math.max(...data.map((item) => item.value), 1)

  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent>
        {data.length ? (
          <div className="flex flex-col gap-3">
            {data.map((item) => (
              <ChartBarRow
                key={item.label}
                label={item.label}
                value={item.value}
                max={maxValue}
                valueFormatter={valueFormatter}
              />
            ))}
          </div>
        ) : (
          <EmptyState
            icon={Database01Icon}
            title={emptyTitle}
            description={emptyDescription}
          />
        )}
      </CardContent>
    </Card>
  )
}

function CapacityChartCard({
  title,
  description,
  data,
}: {
  title: string
  description: string
  data: Array<{ label: string; value: number; max: number }>
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>{description}</CardDescription>
      </CardHeader>
      <CardContent>
        {data.length ? (
          <div className="flex flex-col gap-3">
            {data.map((item) => (
              <ChartBarRow
                key={item.label}
                label={item.label}
                value={item.value}
                max={item.max || 1}
                valueFormatter={(value) =>
                  `${formatNumber(value)}/${formatNumber(item.max)}`
                }
              />
            ))}
          </div>
        ) : (
          <EmptyState
            icon={ServerStack01Icon}
            title="No upstreams"
            description="Upstream load appears after keys are configured."
          />
        )}
      </CardContent>
    </Card>
  )
}

function QuotaPressureCard({ members }: { members: MemberOverview[] }) {
  const quotaMembers = members
    .filter((member) => member.fiveHourQuota > 0 || member.weeklyQuota > 0)
    .sort((left, right) => memberQuotaPressure(right) - memberQuotaPressure(left))
    .slice(0, 12)

  return (
    <Card>
      <CardHeader>
        <CardTitle>Member quota pressure</CardTitle>
        <CardDescription>Highest 5h and weekly credit window usage</CardDescription>
      </CardHeader>
      <CardContent>
        {quotaMembers.length ? (
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
            {quotaMembers.map((member) => (
              <div key={member.id} className="flex flex-col gap-2">
                <div className="flex items-center justify-between gap-3">
                  <span className="truncate text-sm font-medium">{member.name}</span>
                  <StatusBadge status={member.status} />
                </div>
                <MiniQuota
                  label="5h"
                  used={member.fiveHourUsage.credits}
                  quota={member.fiveHourQuota}
                />
                <MiniQuota
                  label="7d"
                  used={member.weeklyUsage.credits}
                  quota={member.weeklyQuota}
                />
              </div>
            ))}
          </div>
        ) : (
          <EmptyState
            icon={UserGroupIcon}
            title="No configured quotas"
            description="Configure member credit quotas to see pressure here."
          />
        )}
      </CardContent>
    </Card>
  )
}

function ChartBarRow({
  label,
  value,
  max,
  valueFormatter,
}: {
  label: string
  value: number
  max: number
  valueFormatter: (value: number) => string
}) {
  return (
    <div className="grid gap-2 sm:grid-cols-[minmax(0,0.8fr)_minmax(0,1.5fr)_auto] sm:items-center">
      <div className="truncate text-sm font-medium">{label}</div>
      <Progress
        value={toPercentValue(ratio(value, max))}
        aria-label={`${label} chart value`}
      />
      <div className="shrink-0 text-right font-mono text-sm">
        {valueFormatter(value)}
      </div>
    </div>
  )
}

function MiniQuota({
  label,
  used,
  quota,
}: {
  label: string
  used: number
  quota: number
}) {
  return (
    <div className="grid grid-cols-[2rem_minmax(0,1fr)_auto] items-center gap-2">
      <div className="text-xs text-muted-foreground">{label}</div>
      <Progress
        value={quota > 0 ? toPercentValue(ratio(used, quota)) : 0}
        aria-label={`${label} quota`}
      />
      <div className="font-mono text-xs">{formatWindowUsage(used, quota)}</div>
    </div>
  )
}

function ChartGrid() {
  return (
    <g stroke="currentColor" className="text-border">
      {[0, 1, 2, 3, 4].map((index) => {
        const y = 20 + index * 55
        return <line key={index} x1="24" x2="700" y1={y} y2={y} strokeWidth="1" />
      })}
    </g>
  )
}

function polylinePoints(
  points: HistoryPoint[],
  key: keyof Omit<HistoryPoint, "generatedAt">,
  maxValue: number
) {
  return points
    .map((point, index) => {
      const x = 24 + (index / Math.max(points.length - 1, 1)) * 676
      const y = 240 - ratio(Number(point[key]) || 0, maxValue) * 220
      return `${x.toFixed(2)},${y.toFixed(2)}`
    })
    .join(" ")
}

function chartStroke(index: number) {
  return ["var(--chart-1)", "var(--chart-3)", "var(--primary)"][index % 3]
}

function memberQuotaPressure(member: MemberOverview) {
  return Math.max(
    member.fiveHourQuota > 0
      ? ratio(member.fiveHourUsage.credits, member.fiveHourQuota)
      : 0,
    member.weeklyQuota > 0
      ? ratio(member.weeklyUsage.credits, member.weeklyQuota)
      : 0
  )
}

function LimitLoadCard({
  title,
  current,
  max,
  icon,
}: {
  title: string
  current: number
  max: number
  icon: IconSvgElement
}) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>
          {formatNumber(current)} active of {formatNumber(max)}
        </CardDescription>
        <CardAction>
          <IconBox icon={icon} />
        </CardAction>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <div className="text-3xl font-medium">
          {max ? formatPercent(current / max) : "0%"}
        </div>
        <Progress
          value={max ? toPercentValue(current / max) : 0}
          aria-label={`${title} ratio`}
        />
      </CardContent>
    </Card>
  )
}

function LimitDetailRow({
  label,
  value,
  meta,
}: {
  label: string
  value: string
  meta?: string
}) {
  return (
    <div className="flex items-start justify-between gap-3">
      <div className="min-w-0">
        <div className="text-sm text-muted-foreground">{label}</div>
        {meta ? <div className="text-xs text-muted-foreground">{meta}</div> : null}
      </div>
      <div className="shrink-0 font-mono text-sm">{value}</div>
    </div>
  )
}

function WindowQuotaRow({
  label,
  used,
  quota,
  meta,
}: {
  label: string
  used: number
  quota: number
  meta: string
}) {
  const configured = quota > 0
  const ratioValue = configured ? ratio(used, quota) : 0

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="text-sm text-muted-foreground">{label}</div>
          <div className="text-xs text-muted-foreground">{meta}</div>
        </div>
        <div className="shrink-0 font-mono text-sm">
          {formatWindowUsage(used, quota)}
        </div>
      </div>
      {configured ? (
        <Progress
          value={toPercentValue(ratioValue)}
          aria-label={`${label} usage`}
        />
      ) : (
        <Badge variant="secondary">not configured</Badge>
      )}
    </div>
  )
}

function StatusCard({
  title,
  active,
  total,
  icon,
}: {
  title: string
  active: number
  total: number
  icon: IconSvgElement
}) {
  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>{title}</CardTitle>
        <CardDescription>
          {formatNumber(active)} active of {formatNumber(total)}
        </CardDescription>
        <CardAction>
          <IconBox icon={icon} />
        </CardAction>
      </CardHeader>
      <CardContent>
        <Progress
          value={total ? toPercentValue(active / total) : 0}
          aria-label={`${title} active ratio`}
        />
      </CardContent>
    </Card>
  )
}

function UsageRow({
  label,
  meta,
  value,
  ratio,
}: {
  label: string
  meta: string
  value: number
  ratio: number
}) {
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <div className="truncate font-medium">{label}</div>
          <div className="text-xs text-muted-foreground">{meta}</div>
        </div>
        <div className="shrink-0 font-mono text-sm">{formatCompact(value)}</div>
      </div>
      <Progress value={toPercentValue(ratio)} aria-label={`${label} usage`} />
    </div>
  )
}

function CapacityRow({ upstream }: { upstream: UpstreamOverview }) {
  const concurrencyRatio = ratio(
    upstream.currentConcurrentRequests,
    upstream.maxConcurrentRequests
  )

  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="truncate font-medium">{upstream.name}</span>
            <StatusBadge status={upstream.healthy ? "healthy" : upstream.status} />
          </div>
          <div className="text-xs text-muted-foreground">
            {upstream.currentConcurrentRequests}/{upstream.maxConcurrentRequests}{" "}
            concurrent · {formatNumber(upstream.failureCount)} failures
          </div>
        </div>
        <div className="flex shrink-0 gap-1">
          {upstream.supportsHttp ? <Badge variant="secondary">HTTP</Badge> : null}
          {upstream.supportsWs ? <Badge variant="secondary">WS</Badge> : null}
        </div>
      </div>
      <Progress
        value={toPercentValue(concurrencyRatio)}
        aria-label={`${upstream.name} capacity`}
      />
    </div>
  )
}

function StatusBadge({ status }: { status: string }) {
  const normalized = status.toLowerCase()
  const variant =
    normalized === "active" ||
    normalized === "healthy" ||
    normalized === "success" ||
    normalized === "normal"
      ? "default"
      : normalized === "disabled" ||
          normalized === "error" ||
          normalized === "over" ||
          /^[45]\d\d$/.test(normalized)
        ? "destructive"
        : "secondary"

  return (
    <Badge variant={variant}>
      {variant === "default" ? (
        <HugeIcon icon={CheckmarkCircle01Icon} data-icon="inline-start" />
      ) : null}
      {status}
    </Badge>
  )
}

function IconBox({ icon }: { icon: IconSvgElement }) {
  return (
    <div className="flex size-9 items-center justify-center rounded-lg bg-muted text-muted-foreground">
      <HugeIcon icon={icon} />
    </div>
  )
}

function HugeIcon({
  icon,
  className,
  ...props
}: {
  icon: IconSvgElement
  className?: string
  "data-icon"?: "inline-start" | "inline-end"
}) {
  return (
    <HugeiconsIcon
      icon={icon}
      strokeWidth={1.8}
      className={cn("shrink-0", className)}
      {...props}
    />
  )
}

function EmptyState({
  icon,
  title,
  description,
}: {
  icon: IconSvgElement
  title: string
  description: string
}) {
  return (
    <Empty className="border">
      <EmptyHeader>
        <EmptyMedia variant="icon">
          <HugeIcon icon={icon} />
        </EmptyMedia>
        <EmptyTitle>{title}</EmptyTitle>
        <EmptyDescription>{description}</EmptyDescription>
      </EmptyHeader>
    </Empty>
  )
}

function DashboardSkeleton() {
  return (
    <div className="flex flex-col gap-6">
      <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
        {Array.from({ length: 4 }).map((_, index) => (
          <Card key={index} size="sm">
            <CardHeader>
              <Skeleton className="h-4 w-32" />
              <Skeleton className="h-3 w-44" />
            </CardHeader>
            <CardContent>
              <Skeleton className="h-9 w-28" />
            </CardContent>
          </Card>
        ))}
      </section>
      <Card>
        <CardHeader>
          <Skeleton className="h-5 w-40" />
          <Skeleton className="h-4 w-64" />
        </CardHeader>
        <CardContent className="flex flex-col gap-3">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </CardContent>
      </Card>
    </div>
  )
}

function ratio(value: number, total: number) {
  return total > 0 ? Math.max(value / total, 0) : 0
}

function toPercentValue(value: number) {
  return Math.min(Math.max(value * 100, 0), 100)
}

function formatNumber(value: number) {
  return numberFormatter.format(value)
}

function formatCompact(value: number) {
  return compactFormatter.format(value)
}

function formatDecimal(value: number) {
  return decimalFormatter.format(value)
}

function formatCredits(value: number) {
  return decimalFormatter.format(value)
}

function formatPercent(value: number) {
  return percentFormatter.format(value)
}

function formatDuration(value: number) {
  if (value < 1000) {
    return `${formatNumber(value)}ms`
  }

  return `${formatDecimal(value / 1000)}s`
}

function formatQuota(value: number) {
  return value > 0 ? formatNumber(value) : "unset"
}

function formatWindowUsage(used: number, quota: number) {
  return quota > 0
    ? `${formatCredits(used)}/${formatNumber(quota)}`
    : formatCredits(used)
}

function formatRefreshInterval(value: number) {
  return `${formatNumber(value / 1000)}s`
}

function formatDateTime(value: string) {
  const date = parseDate(value)
  if (!date) {
    return value
  }

  return date.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  })
}

function formatOptionalDateTime(value: string | null) {
  return value ? formatDateTime(value) : "never"
}

function parseDate(value: string) {
  const normalized = value.includes("T") ? value : value.replace(" ", "T") + "Z"
  const date = new Date(normalized)
  return Number.isNaN(date.getTime()) ? null : date
}

export default App
