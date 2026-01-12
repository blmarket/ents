import { Routes, Route } from 'react-router-dom'
import Layout from './components/Layout'
import HomePage from './pages/HomePage'
import EntityPage from './pages/EntityPage'

function App() {
  return (
    <Routes>
      <Route path="/" element={<Layout />}>
        <Route index element={<HomePage />} />
        <Route path="entities/:id" element={<EntityPage />} />
      </Route>
    </Routes>
  )
}

export default App
