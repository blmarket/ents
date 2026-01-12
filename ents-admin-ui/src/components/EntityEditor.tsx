import { useState, useEffect } from 'react'
import Editor from '@monaco-editor/react'
import type { Entity } from '../api/types'

interface EntityEditorProps {
  entity: Entity
  onSave: (entity: Entity) => Promise<void>
  onCancel: () => void
  saving?: boolean
}

export default function EntityEditor({ entity, onSave, onCancel, saving }: EntityEditorProps) {
  const [jsonValue, setJsonValue] = useState('')
  const [parseError, setParseError] = useState<string | null>(null)

  useEffect(() => {
    setJsonValue(JSON.stringify(entity, null, 2))
    setParseError(null)
  }, [entity])

  const handleChange = (value: string | undefined) => {
    const val = value || ''
    setJsonValue(val)
    try {
      JSON.parse(val)
      setParseError(null)
    } catch (e) {
      setParseError((e as Error).message)
    }
  }

  const handleSave = async () => {
    try {
      const parsed = JSON.parse(jsonValue) as Entity
      await onSave(parsed)
    } catch (e) {
      setParseError((e as Error).message)
    }
  }

  const isDirty = jsonValue !== JSON.stringify(entity, null, 2)

  return (
    <div className="border border-gray-700 rounded overflow-hidden">
      <div className="bg-gray-800 px-3 py-2 border-b border-gray-700 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium text-gray-300">
            Editing: {entity.type}
          </span>
          <span className="text-xs text-gray-500">ID: {entity.id}</span>
          {isDirty && (
            <span className="text-xs text-yellow-500">Modified</span>
          )}
        </div>
        <div className="flex gap-2">
          <button
            onClick={onCancel}
            disabled={saving}
            className="px-3 py-1 text-sm bg-gray-700 hover:bg-gray-600 text-gray-300 rounded transition-colors disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            disabled={!!parseError || saving}
            className="px-3 py-1 text-sm bg-blue-600 hover:bg-blue-700 text-white rounded transition-colors disabled:opacity-50"
          >
            {saving ? 'Saving...' : 'Save'}
          </button>
        </div>
      </div>
      {parseError && (
        <div className="bg-red-900/30 border-b border-red-800 px-3 py-2 text-sm text-red-400">
          JSON Error: {parseError}
        </div>
      )}
      <Editor
        height="400px"
        defaultLanguage="json"
        value={jsonValue}
        theme="vs-dark"
        options={{
          minimap: { enabled: false },
          fontSize: 13,
          lineNumbers: 'on',
          scrollBeyondLastLine: false,
          wordWrap: 'on',
          automaticLayout: true,
        }}
        onChange={handleChange}
      />
    </div>
  )
}
