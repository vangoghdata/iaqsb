# IAQSB

AI-native operations research. Made by IA Nativa.

Code: <https://github.com/vangoghdata/iaqsb> · Installers: <https://github.com/vangoghdata/iaqsb/releases> · [How to install (Spanish)](INSTALAR.md)

Free software (MIT license) in Spanish, English and Portuguese, designed for universities in Latin America: from the first undergraduate operations research course to the doctoral thesis. *[Español: LEEME.md](LEEME.md) · [Português: LEIAME.md](LEIAME.md)*

| Document | Purpose |
| --- | --- |
| [docs/DOCENTES.md](docs/DOCENTES.md) | Guide for instructors: how to use IAQSB in undergraduate, master's and doctoral courses, activities and computer labs |
| [docs/LENGUAJE.md](docs/LENGUAJE.md) | Reference for the algebraic modeling language (with 29 tested models) |
| [docs/FUENTES.md](docs/FUENTES.md) | Bibliography in APA 7 and the origin of each example |
| [COMPATIBILIDAD.md](COMPATIBILIDAD.md) | Compatibility with WinQSB files, module by module |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to report bugs, propose improvements, translate and contribute code |
| [CITATION.cff](CITATION.cff) | How to cite IAQSB |

IAQSB reimplements the 19 modules of WinQSB 2.0 in Rust, opens and saves their files, and explains each solution step by step. It also adds six logistics modules that WinQSB does not have: vehicle routing, transportation mode selection, inventory management, inventory finance, warehouses and logistics KPIs, two production planning modules (the master production schedule and matrix MRP II, Gozinto) and a time series module with modern forecasting (ARIMA, ETS, TBATS). It can be used in three ways, all with the same calculation engine:

- **Desktop application** (Tauri 2 + Svelte) for students and instructors, with a macOS-inspired design: the system font (SF Pro on Mac, Inter on Windows), Apple system colors, and light, dark, automatic and custom themes. The module sidebar appears when the mouse reaches the left edge (or with Ctrl+K) and hides by itself; it can be pinned. Calculation options go in an «Options» panel so they do not take space away from the results. Every chart has buttons to enlarge it (full-screen viewer with zoom and pan), copy it as an image and download it as PNG or SVG, and charts show the value of each series on mouse hover. Entering a cell selects its value, so whatever is typed replaces it.
- **MCP server** (`iaqsb-mcp`), so that Claude or another compatible assistant can formulate and solve problems.
- **Command line** (`iaqsb`), with JSON output for scripts and AI agents without MCP.

## Modules

All 19 WinQSB modules are available. Each report has a summary, structured results, tables and charts, and a **Step by step** tab with the formulas applied to the data.

| WinQSB | IAQSB | Highlights |
| --- | --- | --- |
| LP-ILP | Linear and integer programming | Simplex with tableaus, branch and bound, sensitivity, graphical method; models of any size with the solver (HiGHS, Gurobi, CPLEX, SCIP, CBC or GLPK), semicontinuous and semi-integer variables, multiple objectives with priorities, infeasibility diagnosis (IIS), MPS and LP files |
| NET | Network models | Minimum-cost flow (transshipment) with node costs and capacities and demands with minimum and maximum, transportation, assignment, shortest path, maximum flow, spanning tree, traveling salesman; mathematical model of each network and a button to open it as a linear program |
| PERT-CPM | PERT and critical path | CPM, PERT with probabilities, minimum-cost crashing, Gantt, simulation |
| GP-IGP | Goal programming | Goals with priorities and weights, integer, goal simplex |
| QP-IQP | Quadratic programming | Convex (Clarabel), nonconvex global, integer |
| NLP | Nonlinear programming | Constrained and unconstrained, multiple starts, KKT conditions |
| DP | Dynamic programming | Stagecoach, knapsack, production and inventory |
| AP | Aggregate planning | Production, inventory, overtime and subcontracting strategies, by LP and transportation |
| DA | Decision analysis | Payoff tables (7 criteria, EVPI, EVSI), Bayes, zero-sum games, trees |
| MKP | Markov processes | Classes, steady state, absorption, first passage, evolution |
| ITS | Inventory | EOQ and POQ (with working days, daily production rate and H = i·C), discounts (by % or by the price of each range), newsvendor, dynamic lot sizing (10 methods), (s,Q), (s,S), (R,S), (R,s,S) systems |
| FC | Forecasting and regression | 12 time series methods with parameter search, multiple regression |
| QA | Queueing theory | Exact M/M/s with finite capacity and population, batches, pressure and discouragement; M/G/1, G/G/s, simulation, costs, capacity, sensitivity |
| QSS | Queueing simulation | Customer populations, servers, queues and garbage collectors; 8 input rules (assembly) and 8 output rules (disassembly), 10 disciplines, blocking, batches |
| JOB | Job scheduling | 15 dispatching rules, Johnson, CDS, Palmer, NEH and other flow shop heuristics, 18 criteria, Gantt |
| MRP | Material requirements planning | Low-level-code explosion, 10 lot-sizing rules, scrap, capacity, costs, bill of materials tree |
| FLL | Facility location and layout | Rectilinear, Euclidean and squared Euclidean location; CRAFT-type layout; line balancing (10 rules, optimal, COMSOAL) |
| QCC | Quality control charts | 36 charts (Shewhart, CUSUM, EWMA, moving average, trend, modified, acceptance, standardized), rules, OC curve, capability, histogram with chi-square, probability, Pareto |
| ASA | Acceptance sampling | Single, double, multiple, sequential, ChSP-1, CSP-1, SkSP-2; by variables (mean, k and M methods, sequential); OC, AOQ, ATI, ASN and cost curves; plan search |

Logistics modules added by IAQSB (they do not exist in WinQSB):

| Module | Highlights |
| --- | --- |
| Vehicle routing (VRP) | Depot, customers with demand and vehicles with capacity (and, optionally, maximum distance, limited fleet and costs). Sweep (with angle and direction, or trying every start), Clarke and Wright savings, nearest neighbor and optimal by integer programming; 2-opt improvement; comparison of methods; route map with the sweep zones. Customers by coordinates (Euclidean or rectilinear distance) or street network (the table is completed with the shortest path) |
| Inventory management | Reverse EOQ and POQ (solves in a chain for whatever is missing from what was observed and checks every relationship), ABC classification with Pareto (cutoffs at slope breaks or by cumulative %), reorder point with variable demand (continuous and periodic review, exact or table Z) and day-by-day simulation of the (Q, R) policy with lost sales or back orders, scenarios, inventory chart and search for the best combination |
| Inventory finance | ROI with scenarios and inventory reduction, liquidity (current ratio, acid test, obsolete inventory) and working capital (operating cycle, gap, profit per cycle; also in reverse) with a timeline |
| Warehouses | Palletizing (cases per layer, layers per height, forklift and load strength), pallets per lot, block-storage warehouse of minimum area with a scale floor plan, and sizing of an owned or rented warehouse (month-by-month cost, alternatives, V-shaped curve and optimum by marginal analysis; simple amortization or with the A/P factor) |
| Logistics KPIs | On time, complete, fill rate, order cycle time, documentation, perfect order, OTIF, urgent versus standard (Welch's t-test) and logistics cost over sales versus the standard |
| Transportation mode selection | Transportation-inventory trade-off (Ballou): freight, inventory at the plant, in transit and at the distribution center; composition and trade-off charts, cost as a function of demand and the demand at which two modes cost the same; advantages and disadvantages of each mode |

Production planning modules added by IAQSB:

| Module | Main features |
| --- | --- |
| Master production schedule (MPS) | Basic and assemble-to-order (ATO): demand and planning time fences, discrete and cumulative available to promise, answers to order requests (whether they are accepted, how much can be promised, from when in full and how the MPS changes without touching the frozen zone), rough-cut capacity planning (RCCP) and an optimal MPS by integer programming with the solver, compared with the course rule |
| Matrix MRP II (Gozinto) | Total requirements matrix (or the direct structure, with LM = (I − N)⁻¹), gross and net requirements, net demand, requirements to schedule with surpluses spread over the horizon, machine time versus the capacity of each department, reorder points and quality adjustment |
| Time series (ARIMA, ETS, TBATS) | Exploration (autocorrelation and partial autocorrelation with bounds, Ljung-Box, classical decomposition, trend and seasonal strength, seasonal plot, Box-Cox λ) and forecasting: automatic ARIMA (Hyndman-Khandakar) or given orders, ETS, TBATS (multiple and non-integer seasonality), Theta, Croston (intermittent demand) and naive methods, with intervals, `accuracy()`-style error measures and residual checks; in automatic mode it compares the models on a test period and picks by MASE. **It uses the best engine on the computer:** R with the forecast package (the reference, the one used in class), otherwise Python with statsforecast, otherwise the native Rust engine (anofox-forecast), always available. Exploration and ARIMA coefficients match R |

The exercises from the transportation, distribution and routing classes come as examples (Examples menu): transshipment cases 1, 2 and 3 and problem 1, shortest path (with the binary model), spanning trees, maximum flow, traveling salesman with 4 and 6 nodes, the beverage network for nearest neighbor and cheapest insertion, the sweep for the fresh-food warehouse and the train, truck or plane trade-off. Also those on inventory and warehouses: Ferretería Nacional (EOQ and POQ), the tea and the cleaning solution (Schroeder), the three-range discounts, the POQ assignment, the 15-item ABC, Sell-Rite, the midterm problems, ROI, liquidity and working capital, the bottling plant, the chemical company and workshops 1, 2 and 3 of Group 8. And those on master scheduling and MRP II by professor Arango: the basic master schedule, the assemble-to-order one, product A with its three order requests and the shelves (Gozinto); the seven-item MRP exercise from his spreadsheet is solved exactly like his solution in the MRP module. And the forecasting ones with the fpp2 data he uses in R: usnetelec, h02 (with λ = 0), qcement (ETS versus ARIMA) and gas (TBATS), plus the quarterly profits from his autocorrelation sheet.

## Professional software tools

What is expected from professional optimization software, with the same interface for learning and for research:

- **Algebraic modeling language** (like AMPL, GAMS or LINGO), with keywords in Spanish, English or Portuguese: sets, parameters with data tables, indexed variables, sums with conditions, and general constraints that are linearized automatically (absolute value, maximum and minimum, piecewise linear functions, and/or/not, implications, disjunctions, «at most k of…»), quadratic objective and multiple objectives by priority. The editor checks the model as it is typed (errors with line and column), highlights the syntax, includes examples and shows the solution in tables by index. The LP-ILP sheet can be converted to the language with one button. A nonconvex quadratic objective or one with integer variables is solved with the global search of the QP-IQP module. Full reference: [docs/LENGUAJE.md](docs/LENGUAJE.md).
- **Reports to hand in** in Word (.docx), PDF, HTML, Excel or Markdown: cover page (author, course, institution), formulation, results, tables and figures numbered in APA 7 style, optional step by step, and reference list with hanging indent. IAQSB builds the PDF itself (Word or LibreOffice is not needed).
- **Excel:** paste a table copied from Excel or Google Sheets into any sheet (Ctrl+V), and open and save .xlsx workbooks. A workbook saved by IAQSB carries the complete model and reopens exactly as it was; a hand-made one is read as data for the open module.
- **What if…?** Sliders to change a value and see the solution instantly, a sweep of one value with the chart of the objective, and exact parametric analysis of LP (the objective as a function of a cost or a right-hand side, piecewise, with the points where the basis changes).
- **Practice mode:** the student performs the simplex pivots and the branch and bound branchings, and IAQSB checks each step, gives hints and shows a summary at the end.
- **Automatic solver tuning** (like the *tuning* of Gurobi or CPLEX): tries several configurations on the open model, checks that they give the same result and proposes the fastest one (only if it improves by at least 10 %).
- **Undo and redo** (Ctrl+Z, Ctrl+Y), **autosave** of work on the computer with recovery when reopening, and **recent files**.
- **Visual network editor:** nodes and arcs are added, moved and connected with the mouse; geographic coordinates are shown on a **real map** (OpenStreetMap), and vehicle routing computes distances in km with the haversine formula.
- **Attach exercises to the assistant:** photos or screenshots of the statement (also pasted with Ctrl+V or dragged into the chat), PDFs, Excel tables, CSV and WinQSB or IAQSB files. Images are downscaled automatically; PDFs go as documents to the models that read them; model files arrive already imported, ready to solve or open in the editor. The assistant says which data it read so you can check them before solving.
- **Hierarchical planning chain:** the aggregate plan is disaggregated into master schedules per product and the MPS goes into MRP (core `link` operation; `link_models` tool in MCP and in the assistant).
- **AI assistant inside the app, with the model of your choice:** a chat that sees the open problem and can solve, compare and explain with the same engine, and an **Explain** button on every result. In **Settings → Artificial intelligence → Connect the assistant's API** you pick the provider: OpenRouter (hundreds of models with one key: GPT, Gemini, Claude, Qwen, DeepSeek, Llama, Mistral…), Anthropic (Claude), OpenAI (ChatGPT), Google Gemini, Qwen (Alibaba Cloud), DeepSeek, Mistral, Groq, xAI (Grok), Moonshot (Kimi), free models on your own computer with Ollama or LM Studio, or any server compatible with the OpenAI API (for example, a university's vLLM). The list of models is fetched from the provider, the connection can be tested, and one key per provider is stored, only on the computer. With models that use tools, the assistant solves with the IAQSB engine; if a model does not support them, it says so and answers without them. In the desktop app the requests are made by Rust, so any address works (local servers too). The level (undergraduate or graduate) changes how it explains.
- **Settings** (Ctrl+,): language, level, theme, decimals and decimal separator (comma or point), solver and its parameters, autosave, recent files, report cover page data, AI assistant connection (provider, key and model), keyboard shortcuts and how to cite IAQSB.

## WinQSB files

IAQSB opens and saves the files of all 19 modules in WinQSB format, which the original program can open again. The formats were checked with the **64 sample files that ship with WinQSB 2.0**: all of them are read, solved, saved and read again with the same result (`crates/iaqsb-api/tests/todo/winqsb_samples.rs`). Details by module are in [COMPATIBILIDAD.md](COMPATIBILIDAD.md).

## Optimization engine

Everything solved with linear or integer programming (LP-ILP, goals, transportation, assignment, aggregate planning, optimal routing, line balancing, facility layout…) goes through a single engine, `iaqsb_core::solver`. By default it is **HiGHS** (bundled, free, no size limit): dual and primal simplex, interior point, PDLP, and branch and cut with presolve, cuts and heuristics. If **Gurobi, CPLEX, SCIP, CBC or GLPK** are installed on the computer (the program or its Python package: `gurobipy`, `cplex`, `pyscipopt`), IAQSB detects them and uses them when requested; the license is each user's own. IAQSB does not ship Gurobi or CPLEX because they are commercial, and it makes no promises about their speed: on hard integer problems they are usually faster than HiGHS, and that is what the connector is for.

- **No inherited limits.** WinQSB limits the size of models; IAQSB does not. A 200 × 300 transportation problem (60,000 variables) is solved in a fraction of a second. The only thing that is kept is that the files it saves can be opened in WinQSB: if the model is larger than WinQSB allows, the file is still written (in its algebraic form) and it is then up to WinQSB whether it can open it.
- **Parameters as in Gurobi:** solver (`solver`), time limit (`time_limit`), MIP gap (`mip_gap`, `mip_abs_gap`), threads (`threads`), presolve, method (`auto`, `dual_simplex`, `primal_simplex`, `barrier`, `pdlp`), seed, node and solution limits, tolerances, solver log (`log`) and initial solution (`start`, the *MIP start*). If time runs out, the best solution found is returned with its bound and its gap.
- **Statistics:** every report says which solver was used, how long it took, the model size, the nodes, the bound and the gap.
- **Solver sensitivity with WinQSB semantics:** shadow prices, reduced costs and ranges of c(j) and of the right-hand side, checked against the tableau simplex with 40 random models.
- **Why it is infeasible:** if a model has no solution, the report includes the irreducible infeasible subsystem (IIS: the constraints and bounds that cannot be satisfied at the same time; relaxing any one of them breaks the conflict) and the feasibility relaxation (the smallest change in the right-hand sides that makes it feasible).
- **Multiple objectives** by priority, with a degradation tolerance per level (like Gurobi's `setObjectiveN`); **semicontinuous and semi-integer variables**; **sparse constraints** (`terms`) for large models.
- **Industry formats:** opens and saves MPS and LP (the CPLEX format), in addition to WinQSB's .LPP and JSON. A model too large for the sheet stays as text and is solved directly.
- **The teaching side is kept:** with the default values, small LP-ILP models are solved with WinQSB's tableau simplex and branch and bound (to show the step by step) and the result is checked with the solver; when a solver, a time limit, a gap or threads are requested, the solver is used.

Comparison with IAQSB's tableau simplex (the same method as WinQSB), on random transportation problems, on this test machine (2 cores; `cargo run --profile pruebas -p iaqsb-lp --example banco`):

| Model | Variables × constraints | Tableau simplex | Solver (HiGHS) | Same optimum |
| :-- | --: | --: | --: | :-: |
| Transportation 20 × 30 | 600 × 50 | 0.025 s | 0.002 s | yes |
| Transportation 40 × 60 | 2,400 × 100 | 0.835 s | 0.008 s | yes |
| Transportation 60 × 90 | 5,400 × 150 | 7.08 s | 0.011 s | yes |
| Transportation 200 × 300 | 60,000 × 500 | — | 0.24 s | — |

In integer programming, a location problem with 30 plants and 200 customers (6,030 variables, 30 binary) reaches a gap of 0.08 % in about 8 s, and on one with 12 plants and 60 customers (732 variables) the six solvers (HiGHS, Gurobi, CPLEX, SCIP, CBC and GLPK) give the same optimum.

In the application, the solver button (top bar) chooses the solver and its parameters for all modules, and in LP-ILP it offers **automatic tuning** for the open model. On the CLI: `iaqsb motores`, `iaqsb resolver modelo.mps --motor gurobi --tiempo 60 --brecha 0.01 --hilos 4 --registro`, `iaqsb convertir modelo.lp modelo.LPP` and `iaqsb diagnosticar modelo.lp`. On MCP: `solver_info`, `diagnose_infeasibility`, `convert_model` and the parameters in `options` (LP-ILP) or `options.engine` (the other modules); `solve` accepts `{"format": "lp" | "mps", "text": "…"}` as the model.

## Sources

IAQSB is a research tool: every method, formula, step and example states where it comes from, in **APA 7th edition**.

- Every report has a **Sources** tab with the origin of the data (book, article, class slide or IAQSB original, with chapter or slide), the methods and formulas used with their reference, and the reference list with DOIs. It can be copied in APA or BibTeX.
- The **Examples** menu shows the source of each example.
- Material from the course Taller de Producción y Logística (Industrial Engineering, Universidad Nacional de Colombia) is cited as Sarache (2026a, b, c…); if the slide names a book (Bronson, Ballou, Taha), that book is cited too.
- Every reference is either **verified** (DOI, publisher or catalog) or **to be verified**, and in that case it says what is missing. Pages and editions are never invented.
- The bibliography is in `crates/iaqsb-core/fuentes/bibliografia.json`, the origin of the data of each example in `crates/iaqsb-core/fuentes/ejemplos.json`, and which methods each module cites in `crates/iaqsb-core/fuentes/metodos.json`. [docs/FUENTES.md](docs/FUENTES.md) and [docs/iaqsb.bib](docs/iaqsb.bib) are generated with `python3 tools/generar_fuentes.py`.
- CLI: `iaqsb fuentes`, `iaqsb fuentes --bibtex`, `iaqsb fuentes --pendientes`. MCP: `get_bibliography`; every `solve` returns `references` and `data_source`.
- The tests (`crates/iaqsb-api/tests/todo/fuentes.rs`) require every example of every module to have a source, every report to cite its methods, every citation to exist in the bibliography, and the result published in the source to be reproduced.

All modules and all examples are documented. Examples taken from books give the chapter and section; those still to be verified say what is missing (almost always, confirming the page in a printed copy).

## How it was verified

The algorithms were written from textbooks (Hillier and Lieberman, Taha, Render and Heizer, Montgomery, Winston) and from current libraries (HiGHS, Clarabel, nalgebra, statrs). WinQSB's code was consulted only for the file formats, the default values and the list of functions; the formulas for queueing, simulation, quality control and sampling were taken from its help.

- **232 automated tests** (`cargo test`), with results published in textbooks: Wyndor, Goferbroke (EVPI 142.5), Seervada Park, Hillier's cameras, the emergency room (L = 2), Montgomery's circuit board (c̄ = 19.85, UCL = 33.22), the juice cans (p̄ = 0.2313), Wald's lines (h1 = 1.22, h2 = 1.57), the constants d2, d3 and c4, among many others.
- **Comparisons with independent implementations** in Python (scipy, numpy, HiGHS), with hundreds of random models per module and 0 differences: 400 linear programs, 240 quadratic, 300 goal programs, 645 networks, 300 PERT, 180 dynamic programming, 60 queues against a full Markov chain, 932 acceptance sampling checks, all 36 control charts, 300 transshipment networks with node costs and capacities (against scipy), 80 routing problems against enumeration of all routes, among others.
- **Interface tests** (`apps/desktop/tools`): 593 checks of the practice mode against the core, 84 of pasting from Excel, 29 end-to-end checks in the browser (pasting, exporting the report in all five formats, copying tables and round-tripping through Excel), 47 of the AI provider format and 59 end-to-end checks of the assistant against simulated servers (OpenAI-compatible API and Anthropic API; tools, model list, connection test, model without tools, wrong key and stopping), in the browser and simulating the desktop app. In addition, `cargo test` in `apps/desktop/src-tauri` tests the network requests made from Rust.
- The simulations (queueing, PERT) were checked against the exact formulas where they exist.
- The MCP server was tested with MCP Inspector.

## Structure

```text
crates/iaqsb-core      report types, interface specification, WinQSB reading and writing
crates/iaqsb-lp        linear programming: simplex, branch and bound, sensitivity, graphical method
crates/iaqsb-modules   the other 18 modules and the logistics ones (one directory per module, with its examples)
crates/iaqsb-formats   .LPP files
crates/iaqsb-api       module catalog and JSON calls; used by the three applications
crates/iaqsb-render    report charts in SVG and PNG and Markdown summary for AI
apps/cli               iaqsb (command line)
apps/mcp               iaqsb-mcp (MCP server over stdio)
apps/desktop           desktop application: Svelte + Tauri 2
scripts                builds and installers
```

## Building on Windows

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1
```

The script checks that Rust (MSVC toolchain), Node.js LTS, Microsoft C++ Build Tools, CMake and LLVM are installed, and says how to install whatever is missing:

```powershell
winget install Rustlang.Rustup
winget install OpenJS.NodeJS.LTS
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install Kitware.CMake
winget install LLVM.LLVM
```

After installing, close PowerShell, open a new window and run the script again. It builds the CLI and the MCP server, includes them in the installer and produces a single installer, `IAQSB_0.2.0_x64-setup.exe` (NSIS), in `target\release\bundle\nsis\`. With `-Pruebas` it runs the tests first. The installer carries the IAQSB logo (icon, header and side panel, generated with `python3 tools/instalador_imagenes.py`) and lets the user choose Spanish, English or Portuguese. `.iaqsb` files (algebraic models) and WinQSB files open with a double click.

On Linux or macOS: `scripts/build.sh` (`cmake` and `clang` are also needed; `--pruebas` runs the tests).

**Why it takes long and what was done about it.** It is not because of Rust or Tauri themselves: the first build compiles HiGHS from its C++ source (with CMake), about 500 Rust dependencies (Tauri and its windowing system, chart rendering, Clarabel, nalgebra…) and the final link with cross-module optimization (LTO). To keep it as short as possible:

- Everything shares a single `target/` folder (the CLI, the MCP server and the application), so what was already compiled for one is not compiled again for another.
- Each package's tests go into a single program (there used to be 28 programs, each linked separately).
- `cargo test` uses a development profile: the project's own code compiles quickly and incrementally, and the dependencies, which are compiled only once, are optimized, so numerical tests run at almost final speed. `cargo test --profile pruebas` runs them optimized and without LTO.
- The release build uses «thin» LTO and 16 parallel units (almost the same speed as full LTO, with much less link time).
- The desktop application is compiled only as a Rust library (without the mobile variants, which tripled link time).
- The installer no longer runs the tests by default.

After the first time, a change in the project's own code recompiles only what changed.

To work on the interface with hot reload: `cd apps/desktop`, `npm install` and `npx tauri dev` (first build the CLI and the MCP server and copy them to `apps/desktop/src-tauri/binaries/` as `scripts/build.sh` does; see [CONTRIBUTING.md](CONTRIBUTING.md)).

## Connecting to Claude

After installing, open IAQSB and press **Connect to Claude**: it shows the exact configuration with the path of your installation. It is this block in `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "iaqsb": { "command": "C:\\Users\\<user>\\AppData\\Local\\IAQSB\\iaqsb-mcp.exe" }
  }
}
```

Tools: `list_modules`, `describe_module`, `solve`, `compare_methods`, `get_report_part`, `graphic_method`, `import_winqsb_file`, `export_winqsb_file`, `get_bibliography`, `get_example`, `solver_info`, `diagnose_infeasibility`, `convert_model`, `check_model` (checks a model in the algebraic language), `what_if` (sweep of one value), `parametric_analysis`, `tune_parameters`, `export_report` (Word, PDF, HTML), `export_excel`, `import_excel` and `link_models` (aggregate plan → master schedule → MRP chain). `solve` accepts models in the algebraic language: `{"format": "iaqsb", "text": "…"}`. The examples are also published as MCP resources, and the `resolver_problema` prompt guides Claude in formulating a problem stated in words.

**Designed for AI.** Responses come in Markdown and are short on purpose: the result, the metrics, the main tables, the charts described with their numbers (the arcs of the tour, the values of each series, the routes) and the sources. Large items are not sent unless requested: the step by step, long tables and the equivalent linear model (`data.lp_model`). Every report has a `report_id`, and `get_report_part` requests a section, a step (`step`) or a value (`path`, for example `data.lp_model`). `detail` chooses how much: `summary` (default), `steps` (default when `options.steps` is `true`), `full` or `json` (the whole report). `describe_module` returns the schema and a single example, of the requested problem type (`problem_type`). A 10-node traveling salesman problem, which used to return about 200 KB, now returns about 3 KB.

**Charts as images.** With `"images": true`, `solve`, `compare_methods` and `get_report_part` also return the charts as PNG (networks, tours, plots, Gantt charts, route maps, warehouse floor plans and trees), each with a caption saying which part of the report it comes from. With `"options": {"steps": true}` the step-by-step charts are included: for the traveling salesman, the partial tour after each iteration of nearest neighbor (NNH) or cheapest insertion (CIH). `max_images` limits how many (12 by default). The font is embedded in the program, so the images look the same on any computer.

**Comparing methods.** `compare_methods` solves the same model with each value of the problem type's method option and returns a table with all the metrics, the best one (and ties), the difference of each from the best, a bar chart and the summarized solution of each variant. It works in every module that has several methods: traveling salesman (NNH, CIH, 2-opt, optimal), initial solutions for transportation, minimum spanning tree, vehicle routing, lot sizing, forecasting, aggregate planning, job scheduling, facility layout and inventory management. With `vary` and `values` you choose which option changes and which values; with `variants`, free combinations of options; with `metric`, the metric that decides (for example `MSE` instead of `MAD`). In the application it is the **Compare** button, which appears when the problem type has methods to compare.

## Languages

The whole app is in **Spanish, English and Brazilian Portuguese**. The language is chosen in **Settings** (Ctrl+,) and changes the interface, the reports, the examples, the sources (APA 7 in English: «&», «10th ed.», «n.d.»; in Portuguese: «&» in the reference list, «e» in narrative citations, «10. ed.», «s.d.»; titles are kept in their original language) and the downloaded charts. The change is immediate: the interface changes first and the open problem is solved again in the background (with a notice in the meantime); the core catalogs are prepared when the app opens. The first time, the system language is used.

- Interface: texts are written in Spanish and passed through `t()` (`apps/desktop/src/lib/i18n.svelte.ts`); English is in `apps/desktop/src/lib/i18n/en_*.ts` and Portuguese in `pt*.ts`.
- Core: `crates/iaqsb-core/src/i18n.rs` translates what the API returns with the catalogs `crates/iaqsb-core/i18n/en.json` and `pt.json` (about 6,000 texts and templates extracted from the code and the data with `python3 tools/i18n_extraer.py`) and `en_extra.json` and `pt_extra.json` (single words and phrases built from parts). IDs, keys and codes are not translated, and numerical results are the same in all three languages.
- The algebraic language accepts keywords in all three languages (`minimizar`/`minimize`/`minimizar`, `sujeto a`/`subject to`/`sujeito a`, `suma`/`sum`/`soma`…).
- CLI: `--lang en`, `--lang pt` or `IAQSB_LANG`. MCP: the `language: "en"` or `"pt"` argument in any tool (or `IAQSB_LANG` in the server configuration).
- The test `crates/iaqsb-api/tests/todo/idioma.rs` solves the examples in English and checks that they come out in English, with the same numbers and with their source; `crates/iaqsb-core/src/i18n.rs` tests Portuguese.

To add another language (for example, French for Haiti or the Caribbean) see [CONTRIBUTING.md](CONTRIBUTING.md).

## Command line

```powershell
iaqsb modulos
iaqsb resolver problema.LPP --pasos        # simplex tableaus
iaqsb resolver QUEUE1.QAA                  # any WinQSB file
iaqsb resolver tsp.json --pasos --graficas figuras   # saves the charts as PNG (--svg: also as SVG)
iaqsb comparar tsp.json                    # all methods for the problem type
iaqsb --md resolver tsp.json               # short Markdown summary, for AI agents
iaqsb comparar pronostico.json --metrica MSE --valores single_exponential,linear_trend --graficas figuras
iaqsb importar C_CHART.QCC > modelo.json
iaqsb exportar modelo.json -s copia.QCC
iaqsb --json llamar solve < peticion.json  # full API, for agents
iaqsb --lang en resolver tsp.json         # in English (also: variable IAQSB_LANG=en); --lang pt: Portuguese
iaqsb resolver transporte.iaqsb            # model in the algebraic language
iaqsb revisar transporte.iaqsb             # check only: errors with line and column, and size
iaqsb informe wyndor.LPP -s informe.docx --pasos --autor "Grupo 8" --curso "Taller de Producción y Logística"   # .docx, .pdf, .html, .xlsx or .md
iaqsb convertir wyndor.LPP wyndor.xlsx     # to Excel (and back); also .iaqsb, .mps, .lp, .json
iaqsb barrer wyndor.LPP --dato constraints.2.rhs --desde 10 --hasta 30 --pasos 10   # what if…?
iaqsb parametrico wyndor.LPP --lado Planta3   # objective as a function of the right-hand side of Planta3, piecewise
iaqsb ajustar modelo.mps                   # automatic tuning of the solver parameters
```

The new modules are called `vrp`, `modo`, `inv`, `fin`, `alm` and `kpi`; their files (`.VRP`, `.MOD`, `.INV`, `.FIN`, `.ALM`, `.KPI`) are IAQSB files, not WinQSB files.

The `llamar` operations are `list_modules`, `describe_module`, `spec`, `template`, `solve`, `compare`, `comparable`, `render` (one report chart as SVG or PNG), `graphic`, `import_text`, `export_text`, `examples`, `bibliography`, `translate`, `solver_info`, `diagnose`, `bibtex`, `digest`, `convert`, `check_model`, `sweep`, `parametric`, `tune`, `export_report`, `export_xlsx`, `import_xlsx` and `parse_pasted`; all of them take and return JSON, and with `"lang": "en"` or `"pt"` they respond in that language.

## Models

Each module describes its model with a JSON schema (`iaqsb describir <module>`) and includes examples (`iaqsb --json llamar examples`). For example, linear programming:

```json
{
  "name": "Wyndor",
  "objective": "max",
  "variables": [
    {"name": "X1", "cost": 3},
    {"name": "X2", "cost": 5, "upper": "M", "type": "integer"}
  ],
  "constraints": [
    {"name": "C1", "coefs": [1, 0], "direction": "<=", "rhs": 4},
    {"name": "C2", "coefs": [3, 2], "direction": "<=", "rhs": 18}
  ]
}
```

Infinite bounds are written `"M"` and `"-M"`, as in WinQSB. A model like that one, written in the algebraic language:

```text
maximizar z: 3 X1 + 5 X2;
sujeto a
C1: X1 <= 4;
C2: 3 X1 + 2 X2 <= 18;
```

(in quick mode, without `var`, variables are declared automatically and are nonnegative; see [docs/LENGUAJE.md](docs/LENGUAJE.md)).

## How to cite

If you use IAQSB in academic work, cite the version you used (the details are in [CITATION.cff](CITATION.cff) and in **Settings → About IAQSB**, which also copies it in BibTeX):

IA Nativa. (2026). *IAQSB: investigación de operaciones nativa para IA* (Version 0.2.0) [Software].

Also cite the methods and the data: every report includes its references in APA 7.

## License and credits

Code under the MIT license: it can be used, studied, modified and redistributed, including in courses, computer labs and research projects. Contributions are welcome ([CONTRIBUTING.md](CONTRIBUTING.md)); the community follows the [code of conduct](CODE_OF_CONDUCT.md). IAQSB is an independent reimplementation of WinQSB, whose rights belong to its author, Yih-Long Chang. IAQSB's code contains no WinQSB code.

The folder `crates/iaqsb-api/tests/winqsb` holds the 64 sample files from the WinQSB 2.0 distribution, which are used only in the compatibility tests; if you are going to publish the code, check that your permission covers them or delete that folder (the test can be removed without affecting anything else). Make sure your permission covers the use you want to make of IAQSB.

The charts drawn by the program (MCP server, command line and application downloads) use the Inter (SIL Open Font License 1.1) and DejaVu Sans (Bitstream Vera license) fonts, included with their licenses in `crates/iaqsb-render/fuentes`.
