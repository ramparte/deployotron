import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import "./Dashboard.css";

interface Project {
  id: string;
  name: string;
  git_url: string;
  framework: string;
  created_at: number;
  updated_at: number;
}

function Dashboard() {
  const [projects, setProjects] = useState<Project[]>([]);
  const [loading, setLoading] = useState(true);
  const [showNewProject, setShowNewProject] = useState(false);
  const [newProject, setNewProject] = useState({
    name: "",
    git_url: "",
  });

  useEffect(() => {
    loadProjects();
  }, []);

  async function loadProjects() {
    try {
      setLoading(true);
      const result = await invoke<Project[]>("get_projects");
      setProjects(result);
    } catch (error) {
      console.error("Failed to load projects:", error);
    } finally {
      setLoading(false);
    }
  }

  async function createProject() {
    try {
      await invoke("create_project", {
        name: newProject.name,
        gitUrl: newProject.git_url,
      });
      setNewProject({ name: "", git_url: "" });
      setShowNewProject(false);
      await loadProjects();
    } catch (error) {
      console.error("Failed to create project:", error);
      alert(`Failed to create project: ${error}`);
    }
  }

  async function startDeployment(projectId: string, environment: string) {
    try {
      await invoke("start_deployment", {
        projectId,
        environment,
      });
      alert("Deployment started! Check the Deployments view for progress.");
    } catch (error) {
      console.error("Failed to start deployment:", error);
      alert(`Failed to start deployment: ${error}`);
    }
  }

  if (loading) {
    return <div className="dashboard"><div className="loading">Loading projects...</div></div>;
  }

  return (
    <div className="dashboard">
      <div className="dashboard-header">
        <h2>Projects</h2>
        <button onClick={() => setShowNewProject(true)}>+ New Project</button>
      </div>

      {showNewProject && (
        <div className="new-project-form">
          <h3>Create New Project</h3>
          <input
            type="text"
            placeholder="Project name"
            value={newProject.name}
            onChange={(e) => setNewProject({ ...newProject, name: e.target.value })}
          />
          <input
            type="text"
            placeholder="Git repository URL"
            value={newProject.git_url}
            onChange={(e) => setNewProject({ ...newProject, git_url: e.target.value })}
          />
          <div className="form-actions">
            <button onClick={createProject}>Create</button>
            <button onClick={() => setShowNewProject(false)}>Cancel</button>
          </div>
        </div>
      )}

      <div className="projects-grid">
        {projects.length === 0 ? (
          <div className="empty-state">
            <p>No projects yet. Create your first project to get started!</p>
          </div>
        ) : (
          projects.map((project) => (
            <div key={project.id} className="project-card">
              <h3>{project.name}</h3>
              <p className="project-url">{project.git_url}</p>
              <p className="project-framework">Framework: {project.framework}</p>
              <div className="project-actions">
                <button onClick={() => startDeployment(project.id, "Staging")}>
                  Deploy to Staging
                </button>
                <button onClick={() => startDeployment(project.id, "Production")}>
                  Deploy to Production
                </button>
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

export default Dashboard;
