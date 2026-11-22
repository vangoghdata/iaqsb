# Puente de IAQSB con los motores que se usan desde Python: Gurobi (gurobipy),
# CPLEX (cplex) y SCIP (pyscipopt). IAQSB escribe el modelo en MPS y los
# parámetros en JSON; este programa resuelve y deja el resultado en JSON.
#
#   python externo.py <gurobi|cplex|scip|check> modelo.mps parametros.json salida.json
import json
import sys
import time


def gurobi(mps, P, res):
    import gurobipy as gp
    from gurobipy import GRB

    env = gp.Env(empty=True)
    env.setParam("OutputFlag", 1 if P.get("log") else 0)
    if P.get("log_file"):
        env.setParam("LogFile", P["log_file"])
    env.start()
    m = gp.read(mps, env=env)
    pr = m.Params
    if P.get("time_limit"):
        pr.TimeLimit = P["time_limit"]
    if P.get("mip_gap") is not None:
        pr.MIPGap = P["mip_gap"]
    if P.get("mip_abs_gap") is not None:
        pr.MIPGapAbs = P["mip_abs_gap"]
    if P.get("threads"):
        pr.Threads = P["threads"]
    if P.get("presolve") is not None:
        pr.Presolve = -1 if P["presolve"] else 0
    meth = {"primal_simplex": 0, "dual_simplex": 1, "barrier": 2, "pdlp": 2}.get(P.get("method") or "")
    if meth is not None:
        pr.Method = meth
    if P.get("seed") is not None:
        pr.Seed = int(P["seed"]) % 2000000000
    if P.get("node_limit"):
        pr.NodeLimit = P["node_limit"]
    if P.get("solution_limit"):
        pr.SolutionLimit = P["solution_limit"]
    if P.get("feasibility_tol"):
        pr.FeasibilityTol = P["feasibility_tol"]
    if P.get("integrality_tol"):
        pr.IntFeasTol = P["integrality_tol"]
    start = P.get("start") or {}
    if start:
        for v in m.getVars():
            if v.VarName in start:
                v.Start = start[v.VarName]
    m.optimize()
    st = {
        GRB.OPTIMAL: "optimal", GRB.INFEASIBLE: "infeasible", GRB.INF_OR_UNBD: "infeasible_or_unbounded",
        GRB.UNBOUNDED: "unbounded", GRB.TIME_LIMIT: "time_limit", GRB.NODE_LIMIT: "node_limit",
        GRB.SOLUTION_LIMIT: "solution_limit", GRB.ITERATION_LIMIT: "iteration_limit", GRB.INTERRUPTED: "interrupted",
        GRB.SUBOPTIMAL: "optimal",
    }
    res["status"] = st.get(m.Status, "error")
    res["version"] = "Gurobi %d.%d.%d" % gp.gurobi.version()
    if m.SolCount > 0:
        res["x"] = {v.VarName: v.X for v in m.getVars()}
        res["objective"] = m.ObjVal
    if m.IsMIP:
        try:
            res["bound"] = m.ObjBound
            res["gap"] = m.MIPGap
        except Exception:
            pass
        res["nodes"] = int(m.NodeCount)
    elif m.Status == GRB.OPTIMAL:
        res["duals"] = {c.ConstrName: c.Pi for c in m.getConstrs()}
        res["rc"] = {v.VarName: v.RC for v in m.getVars()}
    res["iterations"] = int(m.IterCount)


def cplex_(mps, P, res):
    import cplex

    c = cplex.Cplex()
    logf = open(P["log_file"], "w") if P.get("log_file") else None
    for setter in (c.set_results_stream, c.set_log_stream, c.set_warning_stream):
        setter(logf)
    c.read(mps)
    pa = c.parameters
    if P.get("time_limit"):
        pa.timelimit.set(P["time_limit"])
    if P.get("mip_gap") is not None:
        pa.mip.tolerances.mipgap.set(P["mip_gap"])
    if P.get("mip_abs_gap") is not None:
        pa.mip.tolerances.absmipgap.set(P["mip_abs_gap"])
    if P.get("threads"):
        pa.threads.set(P["threads"])
    if P.get("presolve") is not None:
        pa.preprocessing.presolve.set(1 if P["presolve"] else 0)
    meth = {"primal_simplex": 1, "dual_simplex": 2, "barrier": 4, "pdlp": 4}.get(P.get("method") or "")
    if meth is not None:
        pa.lpmethod.set(meth)
    if P.get("seed") is not None:
        pa.randomseed.set(int(P["seed"]) % 2000000000)
    if P.get("node_limit"):
        pa.mip.limits.nodes.set(P["node_limit"])
    if P.get("solution_limit"):
        pa.mip.limits.solutions.set(P["solution_limit"])
    if P.get("feasibility_tol"):
        pa.simplex.tolerances.feasibility.set(P["feasibility_tol"])
    if P.get("integrality_tol"):
        pa.mip.tolerances.integrality.set(P["integrality_tol"])
    names = c.variables.get_names()
    start = P.get("start") or {}
    is_mip = c.get_problem_type() not in (c.problem_type.LP, c.problem_type.QP)
    if start and is_mip:
        idx = [i for i, n in enumerate(names) if n in start]
        if idx:
            c.MIP_starts.add(cplex.SparsePair(ind=idx, val=[start[names[i]] for i in idx]), c.MIP_starts.effort_level.auto)
    c.solve()
    s = c.solution
    code = s.get_status()
    st = {1: "optimal", 101: "optimal", 102: "optimal", 2: "unbounded", 118: "unbounded", 3: "infeasible", 103: "infeasible",
          4: "infeasible_or_unbounded", 119: "infeasible_or_unbounded", 11: "time_limit", 107: "time_limit", 108: "time_limit",
          105: "node_limit", 106: "node_limit", 104: "solution_limit", 10: "iteration_limit", 13: "interrupted", 113: "interrupted"}
    res["status"] = st.get(code, "error")
    res["version"] = "CPLEX " + c.get_version()
    try:
        vals = s.get_values()
        res["x"] = dict(zip(names, vals))
        res["objective"] = s.get_objective_value()
    except Exception:
        pass
    if is_mip:
        try:
            res["bound"] = s.MIP.get_best_objective()
            res["gap"] = s.MIP.get_mip_relative_gap()
            res["nodes"] = s.progress.get_num_nodes_processed()
        except Exception:
            pass
    elif res["status"] == "optimal":
        try:
            res["duals"] = dict(zip(c.linear_constraints.get_names(), s.get_dual_values()))
            res["rc"] = dict(zip(names, s.get_reduced_costs()))
        except Exception:
            pass
    try:
        res["iterations"] = int(s.progress.get_num_iterations())
    except Exception:
        pass
    if logf:
        logf.close()


def scip(mps, P, res):
    from pyscipopt import Model

    m = Model()
    m.hideOutput(not P.get("log"))
    if P.get("log_file"):
        m.setLogfile(P["log_file"])
        m.hideOutput(False)
    m.readProblem(mps)
    if P.get("time_limit"):
        m.setParam("limits/time", P["time_limit"])
    if P.get("mip_gap") is not None:
        m.setParam("limits/gap", P["mip_gap"])
    if P.get("mip_abs_gap") is not None:
        m.setParam("limits/absgap", P["mip_abs_gap"])
    if P.get("presolve") is False:
        m.setPresolve(0)
    if P.get("seed") is not None:
        m.setParam("randomization/randomseedshift", int(P["seed"]) % 2000000000)
    if P.get("node_limit"):
        m.setParam("limits/nodes", P["node_limit"])
    if P.get("solution_limit"):
        m.setParam("limits/solutions", P["solution_limit"])
    if P.get("feasibility_tol"):
        m.setParam("numerics/feastol", P["feasibility_tol"])
    start = P.get("start") or {}
    if start:
        sol = m.createSol()
        for v in m.getVars():
            if v.name in start:
                m.setSolVal(sol, v, start[v.name])
        m.addSol(sol, free=True)
    m.optimize()
    st = {"optimal": "optimal", "infeasible": "infeasible", "unbounded": "unbounded", "inforunbd": "infeasible_or_unbounded",
          "timelimit": "time_limit", "nodelimit": "node_limit", "totalnodelimit": "node_limit", "sollimit": "solution_limit",
          "bestsollimit": "solution_limit", "gaplimit": "optimal", "userinterrupt": "interrupted"}
    res["status"] = st.get(m.getStatus(), "error")
    res["version"] = "SCIP " + str(m.version())
    if m.getNSols() > 0:
        best = m.getBestSol()
        res["x"] = {v.name: m.getSolVal(best, v) for v in m.getVars()}
        res["objective"] = m.getObjVal()
    try:
        res["bound"] = m.getDualbound()
        res["gap"] = m.getGap()
        res["nodes"] = m.getNNodes()
        res["iterations"] = m.getNLPIterations()
    except Exception:
        pass


def check(res):
    found = {}
    for name, mod in (("gurobi", "gurobipy"), ("cplex", "cplex"), ("scip", "pyscipopt")):
        try:
            m = __import__(mod)
            if name == "gurobi":
                found[name] = "Gurobi %d.%d.%d" % m.gurobi.version()
            elif name == "cplex":
                found[name] = "CPLEX " + m.Cplex().get_version()
            else:
                found[name] = "SCIP " + str(m.Model().version())
        except Exception:
            pass
    res["found"] = found
    res["status"] = "optimal"


def main():
    solver, mps, params_path, out_path = sys.argv[1:5]
    P = json.load(open(params_path)) if params_path != "-" else {}
    res = {"status": "error", "x": {}, "objective": None, "bound": None, "gap": None, "nodes": 0, "iterations": 0,
           "version": solver, "duals": None, "rc": None, "message": ""}
    t0 = time.time()
    try:
        if solver == "check":
            check(res)
        else:
            {"gurobi": gurobi, "cplex": cplex_, "scip": scip}[solver](mps, P, res)
    except Exception as e:  # licencia, tamaño, modelo...
        res["status"] = "error"
        res["message"] = str(e)
    res["time"] = time.time() - t0
    with open(out_path, "w") as f:
        json.dump(res, f)


if __name__ == "__main__":
    main()
