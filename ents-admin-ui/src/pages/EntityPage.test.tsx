import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, waitFor, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { MemoryRouter, Routes, Route } from 'react-router-dom'
import EntityPage from './EntityPage'
import * as api from '../api'

vi.mock('../api', () => ({
  getEntity: vi.fn(),
  getEntityEdges: vi.fn(),
  getIncomingEdges: vi.fn(),
  updateEntity: vi.fn(),
  deleteEntity: vi.fn(),
  auditEntityEdges: vi.fn(),
  fixEntityEdges: vi.fn(),
  getKnownTypes: vi.fn(),
}))

const mockEntity = {
  id: 1,
  type: 'test_type',
  data: { name: 'Test Entity' },
}

const mockEdges = {
  edges: [{ source: 1, sort_key: 'test_edge', dest: 2 }],
  has_more: false,
}

const mockIncomingEdges = [{ source: 3, sort_key: 'incoming_edge', dest: 1 }]

const mockUpdatedIncomingEdges = [
  { source: 3, sort_key: 'incoming_edge', dest: 1 },
  { source: 4, sort_key: 'new_incoming_edge', dest: 1 },
]

const mockAuditResult = {
  valid: false,
  existing_edges: [],
  expected_edges: [],
  missing: [{ source: 1, sort_key: 'missing_edge', dest: 4 }],
  extra: [],
}

const mockAuditResultValid = {
  valid: true,
  existing_edges: [],
  expected_edges: [],
  missing: [],
  extra: [],
}

function renderEntityPage(entityId: string = '1') {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
        staleTime: 5000,  // Match production config
      },
    },
  })

  // Spy on queryClient methods
  const refetchQueriesSpy = vi.spyOn(queryClient, 'refetchQueries')
  const invalidateQueriesSpy = vi.spyOn(queryClient, 'invalidateQueries')

  render(
    <QueryClientProvider client={queryClient}>
      <MemoryRouter initialEntries={[`/entity/${entityId}`]}>
        <Routes>
          <Route path="/entity/:id" element={<EntityPage />} />
        </Routes>
      </MemoryRouter>
    </QueryClientProvider>
  )

  return { queryClient, refetchQueriesSpy, invalidateQueriesSpy }
}

describe('EntityPage', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(api.getEntity).mockResolvedValue(mockEntity)
    vi.mocked(api.getEntityEdges).mockResolvedValue(mockEdges)
    vi.mocked(api.getIncomingEdges).mockResolvedValue(mockIncomingEdges)
    vi.mocked(api.getKnownTypes).mockResolvedValue(['test_type'])
    vi.mocked(api.auditEntityEdges).mockResolvedValue(mockAuditResult)
    vi.mocked(api.fixEntityEdges).mockResolvedValue(undefined)
  })

  it('renders entity details', async () => {
    renderEntityPage()

    await waitFor(() => {
      expect(screen.getByText('Entity #1')).toBeInTheDocument()
    })
  })

  it('calls refetchQueries (not invalidateQueries) when edges are fixed', async () => {
    const user = userEvent.setup()
    const { refetchQueriesSpy, invalidateQueriesSpy } = renderEntityPage()

    // Wait for entity to load
    await waitFor(() => {
      expect(screen.getByText('Entity #1')).toBeInTheDocument()
    })

    // Click on Audit tab
    const auditTab = screen.getByRole('button', { name: /audit/i })
    await user.click(auditTab)

    // Click Audit Edges button
    const auditButton = await screen.findByRole('button', { name: /audit edges/i })
    await user.click(auditButton)

    // Wait for audit result (showing Fix Edges button)
    const fixButton = await screen.findByRole('button', { name: /fix edges/i })

    // Mock window.confirm to return true
    vi.spyOn(window, 'confirm').mockReturnValue(true)

    // Click Fix Edges button
    await user.click(fixButton)

    // Wait for fix to complete
    await waitFor(() => {
      expect(api.fixEntityEdges).toHaveBeenCalledWith(1, 'test_type')
    })

    // Verify refetchQueries was called for both incoming and outgoing edges
    await waitFor(() => {
      expect(refetchQueriesSpy).toHaveBeenCalledWith({ queryKey: ['edges', 1, 'incoming'] })
      expect(refetchQueriesSpy).toHaveBeenCalledWith({ queryKey: ['edges', 1, 'outgoing'] })
    })
  })

  it('updates incoming edges data after fix edges', async () => {
    const user = userEvent.setup()

    // Set up initial mock - getIncomingEdges returns old data first
    let incomingEdgesCallCount = 0
    vi.mocked(api.getIncomingEdges).mockImplementation(async () => {
      incomingEdgesCallCount++
      // First call returns old data, subsequent calls return updated data
      if (incomingEdgesCallCount === 1) {
        return mockIncomingEdges
      }
      return mockUpdatedIncomingEdges
    })

    // After fix, audit should return valid
    vi.mocked(api.auditEntityEdges)
      .mockResolvedValueOnce(mockAuditResult)
      .mockResolvedValueOnce(mockAuditResultValid)

    renderEntityPage()

    // Wait for entity to load
    await waitFor(() => {
      expect(screen.getByText('Entity #1')).toBeInTheDocument()
    })

    // Click on Incoming tab to verify initial count
    const incomingTab = screen.getByRole('button', { name: /incoming edges/i })
    expect(incomingTab).toHaveTextContent('Incoming Edges (1)')

    // Click on Audit tab
    const auditTab = screen.getByRole('button', { name: /audit/i })
    await user.click(auditTab)

    // Click Audit Edges button
    const auditButton = await screen.findByRole('button', { name: /audit edges/i })
    await user.click(auditButton)

    // Wait for audit result (showing Fix Edges button)
    const fixButton = await screen.findByRole('button', { name: /fix edges/i })

    // Mock window.confirm to return true
    vi.spyOn(window, 'confirm').mockReturnValue(true)

    // Click Fix Edges button
    await user.click(fixButton)

    // Wait for fix to complete and incoming edges to be refetched
    await waitFor(() => {
      expect(api.fixEntityEdges).toHaveBeenCalledWith(1, 'test_type')
    })

    // Wait for the incoming edges query to be refetched (should be called at least twice)
    await waitFor(() => {
      expect(incomingEdgesCallCount).toBeGreaterThanOrEqual(2)
    })

    // Check that the incoming edges tab now shows updated count
    await waitFor(() => {
      const updatedIncomingTab = screen.getByRole('button', { name: /incoming edges/i })
      expect(updatedIncomingTab).toHaveTextContent('Incoming Edges (2)')
    })
  })
})
