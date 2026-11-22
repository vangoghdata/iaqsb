# IAQSB

Pesquisa operacional nativa para IA. Feito por IA Nativa.

Código: <https://github.com/vangoghdata/iaqsb> · Instaladores: <https://github.com/vangoghdata/iaqsb/releases> · [Como instalar (em espanhol)](INSTALAR.md)

Software livre (licença MIT) em espanhol, inglês e português, pensado para as universidades da América Latina: da primeira disciplina de pesquisa operacional na graduação até a tese de doutorado. *[Español: LEEME.md](LEEME.md) · [English: README.md](README.md)*

| Documento | Para quê |
| --- | --- |
| [docs/DOCENTES.md](docs/DOCENTES.md) | Guia para professores: como usar o IAQSB na graduação, no mestrado e no doutorado, atividades e laboratórios de informática |
| [docs/LENGUAJE.md](docs/LENGUAJE.md) | Referência da linguagem de modelagem algébrica (com 29 modelos testados) |
| [docs/FUENTES.md](docs/FUENTES.md) | Bibliografia em APA 7 e origem de cada exemplo |
| [COMPATIBILIDAD.md](COMPATIBILIDAD.md) | Compatibilidade com os arquivos do WinQSB, módulo por módulo |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Como relatar erros, propor melhorias, traduzir e contribuir com código |
| [CITATION.cff](CITATION.cff) | Como citar o IAQSB |

O IAQSB reimplementa em Rust os 19 módulos do WinQSB 2.0, abre e salva os seus arquivos e explica cada solução passo a passo. Além disso, acrescenta seis módulos de logística que o WinQSB não tem: roteirização de veículos, seleção do modal de transporte, gestão de estoques, finanças do estoque, armazéns e indicadores logísticos, dois de planejamento da produção (o programa mestre e o MRP II por matrizes, Gozinto) e um de séries temporais com previsões modernas (ARIMA, ETS, TBATS). Pode ser usado de três maneiras, com o mesmo motor de cálculo:

- **Aplicativo de desktop** (Tauri 2 + Svelte) para estudantes e professores, com um design inspirado no macOS: fonte do sistema (SF Pro no Mac, Inter no Windows), cores do sistema da Apple, temas claro, escuro, automático e personalizado. A barra lateral de módulos aparece ao levar o mouse à borda esquerda (ou com Ctrl+K) e se esconde sozinha; ela pode ser fixada. As opções do cálculo ficam em um painel «Opções» para não tirar espaço dos resultados. Cada gráfico tem botões para ampliá-lo (visualizador em tela cheia com zoom e arraste), copiá-lo como imagem e baixá-lo em PNG ou SVG, e os gráficos mostram o valor de cada série ao passar o mouse. Ao entrar em uma célula, o seu valor é selecionado, de modo que o que se digita o substitui.
- **Servidor MCP** (`iaqsb-mcp`), para que o Claude ou outro assistente compatível formule e resolva problemas.
- **Linha de comando** (`iaqsb`), com saída JSON para scripts e agentes de IA sem MCP.

## Módulos

Os 19 módulos do WinQSB estão disponíveis. Cada relatório traz um resumo, resultados estruturados, tabelas e gráficos, e uma aba **Passo a passo** com as fórmulas aplicadas aos dados.

| WinQSB | IAQSB | Principais recursos |
| --- | --- | --- |
| LP-ILP | Programação linear e inteira | Simplex com quadros, branch and bound, sensibilidade, método gráfico; modelos de qualquer tamanho com o solver (HiGHS, Gurobi, CPLEX, SCIP, CBC ou GLPK), variáveis semicontínuas e semi-inteiras, vários objetivos com prioridades, diagnóstico de inviabilidade (IIS), arquivos MPS e LP |
| NET | Modelos de redes | Fluxo de custo mínimo (transbordo) com custo e capacidade nos nós e demandas com mínimo e máximo, transporte, designação, caminho mais curto, fluxo máximo, árvore geradora, caixeiro-viajante; modelo matemático de cada rede e botão para abri-lo como programa linear |
| PERT-CPM | PERT e caminho crítico | CPM, PERT com probabilidades, compressão de custo mínimo, Gantt, simulação |
| GP-IGP | Programação por metas | Metas com prioridades e pesos, inteiras, simplex de metas |
| QP-IQP | Programação quadrática | Convexa (Clarabel), não convexa global, inteira |
| NLP | Programação não linear | Com e sem restrições, vários pontos de partida, condições KKT |
| DP | Programação dinâmica | Diligência, mochila, produção e estoque |
| AP | Planejamento agregado | Estratégias de produção, estoque, horas extras e subcontratação, por PL e transporte |
| DA | Análise de decisão | Tabelas de payoffs (7 critérios, VEIP, VEIA), Bayes, jogos de soma zero, árvores |
| MKP | Processos de Markov | Classes, estado estacionário, absorção, primeira passagem, evolução |
| ITS | Estoques | EOQ e POQ (com dias úteis, taxa de produção diária e H = i·C), descontos (por % ou pelo preço de cada faixa), jornaleiro, lotes dinâmicos (10 métodos), sistemas (s,Q), (s,S), (R,S), (R,s,S) |
| FC | Previsões e regressão | 12 métodos de séries temporais com busca de parâmetros, regressão múltipla |
| QA | Teoria das filas | M/M/s exato com capacidade e população finitas, lotes, pressão e desistência; M/G/1, G/G/s, simulação, custos, capacidade, sensibilidade |
| QSS | Simulação de filas | Populações, servidores, filas e coletores; 8 regras de entrada (montagem) e 8 de saída (desmontagem), 10 disciplinas, bloqueio, lotes |
| JOB | Programação de tarefas | 15 regras de sequenciamento, Johnson, CDS, Palmer, NEH e outras heurísticas de flow shop, 18 critérios, Gantt |
| MRP | Planejamento das necessidades de materiais | Explosão por nível mais baixo, 10 regras de lote, refugo, capacidade, custos, árvore de materiais |
| FLL | Localização e arranjo físico | Localização retilínea, euclidiana e euclidiana ao quadrado; arranjo físico tipo CRAFT; balanceamento de linha (10 regras, ótimo, COMSOAL) |
| QCC | Gráficos de controle de qualidade | 36 gráficos (Shewhart, CUSUM, EWMA, médias móveis, tendência, modificados, de aceitação, padronizados), regras, curva CO, capacidade, histograma com qui-quadrado, probabilidade, Pareto |
| ASA | Amostragem de aceitação | Simples, dupla, múltipla, sequencial, ChSP-1, CSP-1, SkSP-2; por variáveis (média, métodos k e M, sequencial); curvas CO, AOQ, ATI, ASN e de custo; busca do plano |

Módulos de logística que o IAQSB acrescenta (não existem no WinQSB):

| Módulo | Principais recursos |
| --- | --- |
| Roteirização de veículos (VRP) | Depósito, clientes com demanda e veículos com capacidade (e, se desejado, distância máxima, frota limitada e custos). Varredura (com ângulo e sentido, ou testando todos os inícios), economias de Clarke e Wright, vizinho mais próximo e ótimo por programação inteira; melhoria 2-opt; comparação de métodos; mapa de rotas com as zonas da varredura. Clientes por coordenadas (distância euclidiana ou retilínea) ou rede de ruas (a tabela é completada com o caminho mais curto) |
| Gestão de estoques | EOQ e POQ ao contrário (calcula em cadeia o que falta a partir do que foi observado e verifica todas as relações), classificação ABC com Pareto (cortes pela quebra de inclinação ou por % acumulado), ponto de pedido com demanda variável (revisão contínua e periódica, Z exato ou de tabela) e simulação dia a dia da política (Q, R) com vendas perdidas ou back orders, cenários, gráfico do estoque e busca da melhor combinação |
| Finanças do estoque | ROI com cenários e redução de estoque, liquidez (liquidez corrente, liquidez seca, estoque obsoleto) e capital de giro (ciclo operacional, defasagem, lucro por ciclo; também ao contrário) com linha do tempo |
| Armazéns | Paletização (caixas por camada, camadas por altura, empilhadeira e resistência), paletes por lote, armazém em blocos de área mínima com planta em escala, e dimensionamento do armazém próprio ou alugado (custo mês a mês, alternativas, curva em V e ótimo por análise marginal; amortização simples ou com fator A/P) |
| Indicadores logísticos | No prazo, completos, fill rate, ciclo do pedido, documentação, pedido perfeito, OTIF, urgentes frente a padrão (teste t de Welch) e custo logístico sobre vendas frente ao padrão |
| Seleção do modal de transporte | Trade-off transporte-estoques (Ballou): frete, estoque na fábrica, em trânsito e no centro de distribuição; gráficos de composição e de trade-off, custo segundo a demanda e demanda em que dois modais custam o mesmo; vantagens e desvantagens de cada modal |

Módulos de planejamento da produção que o IAQSB acrescenta:

| Módulo | O principal |
| --- | --- |
| Programa mestre (PMP) | Básico e de montagem sob encomenda (ATO): barreiras de tempo de demanda e de planejamento, disponível para promessa (ATP) discreto e acumulado, resposta a solicitações de pedidos (se são aceitas, quanto se pode prometer, desde quando completas e como fica o PMP sem tocar no congelado), plano aproximado de capacidade (RCCP) e PMP ótimo por programação inteira com o solver, comparado com a regra do curso |
| MRP II por matrizes (Gozinto) | Matriz de necessidades totais (ou a estrutura direta, com LM = (I − N)⁻¹), necessidades brutas e líquidas, demanda líquida, necessidades a programar com as sobras repartidas no horizonte, tempos de máquina contra a capacidade de cada departamento, pontos de pedido e ajuste por qualidade |
| Séries temporais (ARIMA, ETS, TBATS) | Exploração (autocorrelação e autocorrelação parcial com bandas, Ljung-Box, decomposição clássica, força da tendência e da sazonalidade, gráfico sazonal, λ de Box-Cox) e previsões: ARIMA automático (Hyndman-Khandakar) ou com ordens dadas, ETS, TBATS (várias sazonalidades e não inteiras), Theta, Croston (demanda intermitente) e ingênuos, com intervalos, medidas de erro como `accuracy()` e revisão de resíduos; no modo automático compara os modelos em um período de teste e escolhe pelo MASE. **Usa o melhor motor que houver no computador:** R com o pacote forecast (a referência, a do curso), senão Python com statsforecast, senão o motor nativo em Rust (anofox-forecast), sempre disponível. A exploração e os coeficientes ARIMA coincidem com o R |

Os exercícios das aulas de transporte, distribuição e roteirização vêm como exemplos (menu Exemplos): transbordo casos 1, 2 e 3 e o problema 1, caminho mais curto (com o modelo binário), árvores geradoras, fluxo máximo, caixeiro-viajante de 4 e 6 nós, a rede de bebidas para o vizinho mais próximo e a inserção mais barata, a varredura do armazém de alimentos frescos e o trade-off trem, caminhão ou avião. Também os de estoques e armazéns: Ferretería Nacional (EOQ e POQ), o chá e a solução de limpeza (Schroeder), os descontos de três faixas, a tarefa do POQ, o ABC de 15 itens, Sell-Rite, os problemas da prova parcial, o ROI, a liquidez e o capital de giro, a fábrica envasadora, a companhia química e as oficinas 1, 2 e 3 do Grupo 8. E os de programa mestre e MRP II do professor Arango: o programa mestre básico, o de montagem sob encomenda, o produto A com as suas três solicitações de pedidos e as estantes (Gozinto); o exercício MRP de sete itens da sua planilha é resolvido igual à sua solução no módulo MRP. E os de previsão com os dados do fpp2 que ele usa no R: usnetelec, h02 (com λ = 0), qcement (ETS contra ARIMA) e gas (TBATS), além dos lucros trimestrais da sua planilha de autocorrelação.

## Ferramentas de software profissional

O que se espera de um programa de otimização profissional, com a mesma interface para aprender e para pesquisar:

- **Linguagem de modelagem algébrica** (como AMPL, GAMS ou LINGO), com palavras-chave em espanhol, inglês ou português: conjuntos, parâmetros com tabelas de dados, variáveis indexadas, somatórios com condições e restrições gerais que são linearizadas automaticamente (valor absoluto, máximo e mínimo, funções lineares por partes, e/ou/não, implicações, disjunções, «no máximo k de…»), objetivo quadrático e vários objetivos por prioridade. O editor verifica o modelo enquanto ele é digitado (erros com linha e coluna), destaca a sintaxe, traz exemplos e mostra a solução em tabelas por índices. A planilha do LP-ILP passa para a linguagem com um botão. Um objetivo quadrático não convexo ou com variáveis inteiras é resolvido com a busca global do módulo QP-IQP. Referência completa: [docs/LENGUAJE.md](docs/LENGUAJE.md).
- **Relatórios para entregar** em Word (.docx), PDF, HTML, Excel ou Markdown: capa (autor, disciplina, instituição), formulação, resultados, tabelas e figuras numeradas no estilo APA 7, passo a passo opcional e lista de referências com recuo deslocado. O PDF é gerado pelo próprio IAQSB (não é preciso ter Word nem LibreOffice).
- **Excel:** colar em qualquer planilha uma tabela copiada do Excel ou do Google Planilhas (Ctrl+V), e abrir e salvar pastas de trabalho .xlsx. Uma pasta de trabalho salva pelo IAQSB traz o modelo completo e é reaberta exatamente igual; uma feita à mão é lida como dados do módulo aberto.
- **E se…?** Controles deslizantes para mudar um dado e ver a solução na hora, varredura de um dado com o gráfico do objetivo, e análise paramétrica exata de PL (o objetivo como função de um custo ou de um lado direito, por partes, com os pontos onde a base muda).
- **Modo prática:** o estudante faz os pivoteamentos do simplex e as ramificações do branch and bound, e o IAQSB confere cada passo, dá dicas e mostra o resumo no final.
- **Ajuste automático do solver** (como o *tuning* do Gurobi ou do CPLEX): testa várias configurações no modelo aberto, verifica que deem o mesmo resultado e propõe a mais rápida (só se ela melhorar pelo menos 10%).
- **Desfazer e refazer** (Ctrl+Z, Ctrl+Y), **salvamento automático** do trabalho no computador com recuperação ao abrir de novo, e **arquivos recentes**.
- **Editor visual de redes:** nós e arcos são adicionados, movidos e conectados com o mouse; as coordenadas geográficas aparecem em um **mapa real** (OpenStreetMap), e a roteirização de veículos calcula distâncias em km com a fórmula de haversine.
- **Anexar exercícios ao assistente:** fotos ou capturas do enunciado (também coladas com Ctrl+V ou arrastadas para o chat), PDF, tabelas do Excel, CSV e arquivos do WinQSB ou do IAQSB. As imagens são reduzidas sozinhas; os PDF vão como documento para os modelos que os leem; os arquivos de modelos chegam já importados, prontos para resolver ou abrir no editor. O assistente diz quais dados leu para que você os revise antes de resolver.
- **Cadeia hierárquica de planejamento:** o plano agregado é desagregado em programas mestres por produto e o PMP passa ao MRP (operação `link` do núcleo; ferramenta `link_models` do MCP e do assistente).
- **Assistente de IA dentro do aplicativo, com o modelo que você escolher:** um chat que vê o problema aberto e pode resolver, comparar e explicar com o mesmo solver, e o botão **Explicar** em cada resultado. Em **Configurações → Inteligência artificial → Conectar a API do assistente** escolhe-se o provedor: OpenRouter (centenas de modelos com uma só chave: GPT, Gemini, Claude, Qwen, DeepSeek, Llama, Mistral…), Anthropic (Claude), OpenAI (ChatGPT), Google Gemini, Qwen (Alibaba Cloud), DeepSeek, Mistral, Groq, xAI (Grok), Moonshot (Kimi), modelos grátis no próprio computador com Ollama ou LM Studio, ou qualquer servidor compatível com a API da OpenAI (por exemplo, o vLLM de uma universidade). A lista de modelos vem do provedor, dá para testar a conexão e guarda-se uma chave por provedor, só no computador. Com os modelos que usam ferramentas, o assistente resolve com o solver do IAQSB; se um modelo não as aceita, ele avisa e responde sem elas. No aplicativo de desktop as requisições são feitas pelo Rust, então funciona com qualquer endereço (também servidores locais). O nível (graduação ou pós-graduação) muda a forma de explicar.
- **Configurações** (Ctrl+,): idioma, nível, tema, casas decimais e separador decimal (vírgula ou ponto), solver e seus parâmetros, salvamento automático, arquivos recentes, dados da capa dos relatórios, conexão do assistente de IA (provedor, chave e modelo), atalhos de teclado e como citar o IAQSB.

## Arquivos do WinQSB

O IAQSB abre e salva os arquivos dos 19 módulos no formato do WinQSB, que o programa original consegue abrir de novo. Os formatos foram verificados com os **64 arquivos de exemplo que acompanham o WinQSB 2.0**: todos são lidos, resolvidos, salvos e lidos de novo com o mesmo resultado (`crates/iaqsb-api/tests/todo/winqsb_samples.rs`). O detalhe por módulo está em [COMPATIBILIDAD.md](COMPATIBILIDAD.md).

## Motor de otimização

Tudo o que é resolvido com programação linear ou inteira (LP-ILP, metas, transporte, designação, planejamento agregado, roteirização ótima, balanceamento de linha, arranjo físico…) passa por um único solver, `iaqsb_core::solver`. Por padrão é o **HiGHS** (incluído, livre, sem limite de tamanho): simplex dual e primal, pontos interiores, PDLP e branch and cut com presolve, cortes e heurísticas. Se **Gurobi, CPLEX, SCIP, CBC ou GLPK** estiverem instalados no computador (o programa ou o seu pacote de Python: `gurobipy`, `cplex`, `pyscipopt`), o IAQSB os detecta e os usa quando solicitados; a licença é a de cada usuário. O IAQSB não traz Gurobi nem CPLEX porque são comerciais, e não promete a velocidade deles: em problemas inteiros difíceis costumam ser mais rápidos que o HiGHS, e é para isso que existe o conector.

- **Sem limites herdados.** O WinQSB limita o tamanho dos modelos; o IAQSB não. Um transporte de 200 × 300 (60.000 variáveis) é resolvido em uma fração de segundo. A única coisa que se mantém é que os arquivos salvos possam ser abertos no WinQSB: se o modelo for maior do que o WinQSB admite, o arquivo é escrito mesmo assim (na sua forma algébrica) e passa a depender do WinQSB conseguir abri-lo.
- **Parâmetros como no Gurobi:** solver (`solver`), tempo máximo (`time_limit`), gap MIP (`mip_gap`, `mip_abs_gap`), threads (`threads`), presolve, método (`auto`, `dual_simplex`, `primal_simplex`, `barrier`, `pdlp`), semente, limites de nós e de soluções, tolerâncias, log do solver (`log`) e solução inicial (`start`, o *MIP start*). Se o tempo acabar, é devolvida a melhor solução encontrada com o seu limitante e o seu gap.
- **Estatísticas:** cada relatório diz qual solver resolveu, em quanto tempo, o tamanho do modelo, os nós, o limitante e o gap.
- **Sensibilidade do solver com a semântica do WinQSB:** preços sombra, custos reduzidos e intervalos de c(j) e do lado direito, conferidos contra o simplex de quadros com 40 modelos aleatórios.
- **Por que é inviável:** se um modelo não tem solução, o relatório traz o subsistema irredutível inviável (IIS: as restrições e limites que não podem ser cumpridos ao mesmo tempo; basta relaxar um deles para romper o conflito) e a relaxação de viabilidade (a menor mudança dos lados direitos que o torna viável).
- **Vários objetivos** por prioridades, com tolerância de degradação por nível (como `setObjectiveN` do Gurobi); **variáveis semicontínuas e semi-inteiras**; **restrições esparsas** (`terms`) para modelos grandes.
- **Formatos da indústria:** abre e salva MPS e LP (o do CPLEX), além do .LPP do WinQSB e de JSON. Um modelo grande demais para a planilha fica como texto e é resolvido diretamente.
- **O lado didático se mantém:** com os valores padrão, os modelos pequenos de LP-ILP são resolvidos com o simplex de quadros e o branch and bound do WinQSB (para ver o passo a passo) e o resultado é conferido com o solver; ao pedir um solver, um tempo, um gap ou threads, usa-se o solver.

Comparação com o simplex de quadros do IAQSB (o mesmo método do WinQSB), em problemas de transporte aleatórios, neste computador de testes (2 núcleos; `cargo run --profile pruebas -p iaqsb-lp --example banco`):

| Modelo | Variáveis × restrições | Simplex de quadros | Solver (HiGHS) | Mesmo ótimo |
| :-- | --: | --: | --: | :-: |
| Transporte 20 × 30 | 600 × 50 | 0,025 s | 0,002 s | sim |
| Transporte 40 × 60 | 2.400 × 100 | 0,835 s | 0,008 s | sim |
| Transporte 60 × 90 | 5.400 × 150 | 7,08 s | 0,011 s | sim |
| Transporte 200 × 300 | 60.000 × 500 | — | 0,24 s | — |

Em programação inteira, uma localização de 30 fábricas e 200 clientes (6.030 variáveis, 30 binárias) chega a um gap de 0,08% em cerca de 8 s, e em uma de 12 fábricas e 60 clientes (732 variáveis) os seis solvers (HiGHS, Gurobi, CPLEX, SCIP, CBC e GLPK) dão o mesmo ótimo.

No aplicativo, o botão do solver (barra superior) escolhe o solver e os seus parâmetros para todos os módulos, e no LP-ILP oferece o **ajuste automático** para o modelo aberto. Na CLI: `iaqsb motores`, `iaqsb resolver modelo.mps --motor gurobi --tiempo 60 --brecha 0.01 --hilos 4 --registro`, `iaqsb convertir modelo.lp modelo.LPP` e `iaqsb diagnosticar modelo.lp`. No MCP: `solver_info`, `diagnose_infeasibility`, `convert_model` e os parâmetros em `options` (LP-ILP) ou `options.engine` (os demais módulos); `solve` aceita `{"format": "lp" | "mps", "text": "…"}` como modelo.

## Fontes

O IAQSB é uma ferramenta de pesquisa: cada método, fórmula, passo e exemplo diz de onde vem, em **APA 7.ª edição**.

- Cada relatório tem uma aba **Fontes** com a origem dos dados (livro, artigo, slide de aula ou original do IAQSB, com capítulo ou slide), os métodos e fórmulas usados com a sua referência, e a lista de referências com o DOI. Pode ser copiada em APA ou em BibTeX.
- O menu **Exemplos** mostra a fonte de cada exemplo.
- O que vem da disciplina Taller de Producción y Logística (Engenharia Industrial, Universidad Nacional de Colombia) é citado como Sarache (2026a, b, c…); se o slide menciona um livro (Bronson, Ballou, Taha), ele também é citado.
- Cada referência está **verificada** (DOI, editora ou catálogo) ou **a verificar**, e nesse caso diz-se o que falta. Não se inventam páginas nem edições.
- A bibliografia está em `crates/iaqsb-core/fuentes/bibliografia.json`, a origem dos dados de cada exemplo em `crates/iaqsb-core/fuentes/ejemplos.json` e quais métodos cada módulo cita em `crates/iaqsb-core/fuentes/metodos.json`. [docs/FUENTES.md](docs/FUENTES.md) e [docs/iaqsb.bib](docs/iaqsb.bib) são gerados com `python3 tools/generar_fuentes.py`.
- CLI: `iaqsb fuentes`, `iaqsb fuentes --bibtex`, `iaqsb fuentes --pendientes`. MCP: `get_bibliography`; cada `solve` traz `references` e `data_source`.
- Os testes (`crates/iaqsb-api/tests/todo/fuentes.rs`) exigem que cada exemplo de todos os módulos tenha fonte, que cada relatório cite os seus métodos, que cada citação exista na bibliografia e que o resultado publicado na fonte seja reproduzido.

Todos os módulos e todos os exemplos estão documentados. Os exemplos tirados de livros indicam capítulo e seção; os que continuam a verificar dizem o que falta (quase sempre, confirmar a página em um exemplar impresso).

## Como foi verificado

Os algoritmos foram escritos a partir de livros-texto (Hillier e Lieberman, Taha, Render e Heizer, Montgomery, Winston) e de bibliotecas atuais (HiGHS, Clarabel, nalgebra, statrs). O código do WinQSB só foi consultado para os formatos de arquivo, os valores padrão e a lista de funções; as fórmulas de filas, simulação, controle de qualidade e amostragem foram tiradas da sua ajuda.

- **232 testes automáticos** (`cargo test`), com resultados publicados nos livros: Wyndor, Goferbroke (VEIP 142,5), Parque Seervada, as câmeras de Hillier, o pronto-socorro (L = 2), a placa de circuito de Montgomery (c̄ = 19,85, LSC = 33,22), as latas de suco (p̄ = 0,2313), as retas de Wald (h1 = 1,22, h2 = 1,57), as constantes d2, d3 e c4, entre muitos outros.
- **Comparações com implementações independentes** em Python (scipy, numpy, HiGHS), com centenas de modelos aleatórios por módulo e 0 diferenças: 400 programas lineares, 240 quadráticos, 300 de metas, 645 de redes, 300 de PERT, 180 de programação dinâmica, 60 filas contra uma cadeia de Markov completa, 932 verificações de amostragem de aceitação, os 36 gráficos de controle, 300 redes de transbordo com custo e capacidade nos nós (contra o scipy), 80 problemas de roteirização contra a enumeração de todas as rotas, entre outros.
- **Testes da interface** (`apps/desktop/tools`): 593 verificações do modo prática contra o núcleo, 84 da colagem do Excel, 29 de ponta a ponta no navegador (colar, exportar o relatório nos cinco formatos, copiar tabelas e ida e volta pelo Excel), 47 do formato dos provedores de IA e 59 de ponta a ponta do assistente contra servidores simulados (API compatível com OpenAI e API da Anthropic; ferramentas, lista de modelos, teste de conexão, modelo sem ferramentas, chave errada e interromper), no navegador e simulando o aplicativo de desktop. Além disso, `cargo test` em `apps/desktop/src-tauri` testa as requisições de rede feitas pelo Rust.
- As simulações (filas, PERT) foram conferidas contra as fórmulas exatas quando existem.
- O servidor MCP foi testado com o MCP Inspector.

## Estrutura

```text
crates/iaqsb-core      tipos do relatório, especificação da interface, leitura e escrita do WinQSB
crates/iaqsb-lp        programação linear: simplex, branch and bound, sensibilidade, método gráfico
crates/iaqsb-modules   os outros 18 módulos e os de logística (um diretório por módulo, com os seus exemplos)
crates/iaqsb-formats   arquivos .LPP
crates/iaqsb-api       catálogo de módulos e chamadas JSON; usado pelos três aplicativos
crates/iaqsb-render    gráficos dos relatórios em SVG e PNG e resumo em Markdown para a IA
apps/cli               iaqsb (linha de comando)
apps/mcp               iaqsb-mcp (servidor MCP por stdio)
apps/desktop           aplicativo de desktop: Svelte + Tauri 2
scripts                compilação e instaladores
```

## Compilar no Windows

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1
```

O script verifica se estão instalados Rust (toolchain MSVC), Node.js LTS, Microsoft C++ Build Tools, CMake e LLVM, e diz como instalar o que faltar:

```powershell
winget install Rustlang.Rustup
winget install OpenJS.NodeJS.LTS
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
winget install Kitware.CMake
winget install LLVM.LLVM
```

Depois de instalar, feche o PowerShell, abra uma janela nova e execute o script de novo. Ele compila a CLI e o servidor MCP, inclui ambos no instalador e gera um único instalador, `IAQSB_0.2.0_x64-setup.exe` (NSIS), em `target\release\bundle\nsis\`. Com `-Pruebas`, executa antes os testes. O instalador leva o logo do IAQSB (ícone, cabeçalho e painel lateral, gerados com `python3 tools/instalador_imagenes.py`) e permite escolher espanhol, inglês ou português. Os arquivos `.iaqsb` (modelos algébricos) e os do WinQSB abrem com clique duplo.

No Linux ou no macOS: `scripts/build.sh` (são necessários também `cmake` e `clang`; `--pruebas` executa os testes).

**Por que demora e o que foi feito.** Não é por causa do Rust nem do Tauri em si: a primeira compilação constrói o HiGHS a partir do seu código em C++ (com CMake), cerca de 500 dependências de Rust (o Tauri e o seu sistema de janelas, a renderização de gráficos, Clarabel, nalgebra…) e a ligação final com otimização entre módulos (LTO). Para que demore o mínimo possível:

- Tudo compartilha uma única pasta `target/` (a CLI, o servidor MCP e o aplicativo), de modo que o que já foi compilado para um não é compilado de novo para outro.
- Os testes de cada pacote ficam em um único programa (antes eram 28 programas, cada um ligado separadamente).
- `cargo test` usa um perfil de desenvolvimento: o código próprio é compilado rápido e por partes, e as dependências, que são compiladas uma única vez, ficam otimizadas, de modo que os testes numéricos rodam quase na velocidade final. `cargo test --profile pruebas` os roda otimizados e sem LTO.
- A versão final usa LTO «thin» e 16 unidades em paralelo (quase a mesma velocidade do LTO completo, com muito menos tempo de ligação).
- O aplicativo de desktop é compilado apenas como biblioteca de Rust (sem as variantes para dispositivos móveis, que triplicavam a ligação).
- O instalador já não executa os testes por padrão.

Depois da primeira vez, uma mudança no código próprio recompila só o que mudou.

Para trabalhar na interface com recarga automática: `cd apps/desktop`, `npm install` e `npx tauri dev` (antes, compile a CLI e o servidor MCP e copie-os para `apps/desktop/src-tauri/binaries/` como faz o `scripts/build.sh`; veja [CONTRIBUTING.md](CONTRIBUTING.md)).

## Conectar ao Claude

Depois de instalar, abra o IAQSB e clique em **Conectar ao Claude**: ele mostra a configuração exata com o caminho da sua instalação. É este bloco em `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "iaqsb": { "command": "C:\\Users\\<usuário>\\AppData\\Local\\IAQSB\\iaqsb-mcp.exe" }
  }
}
```

Ferramentas: `list_modules`, `describe_module`, `solve`, `compare_methods`, `get_report_part`, `graphic_method`, `import_winqsb_file`, `export_winqsb_file`, `get_bibliography`, `get_example`, `solver_info`, `diagnose_infeasibility`, `convert_model`, `check_model` (verifica um modelo da linguagem algébrica), `what_if` (varredura de um dado), `parametric_analysis`, `tune_parameters`, `export_report` (Word, PDF, HTML), `export_excel`, `import_excel` e `link_models` (cadeia plano agregado → programa mestre → MRP). `solve` aceita modelos da linguagem algébrica: `{"format": "iaqsb", "text": "…"}`. Os exemplos também são publicados como recursos MCP, e o prompt `resolver_problema` orienta o Claude a formular um problema escrito em palavras.

**Pensado para a IA.** As respostas vêm em Markdown e são curtas de propósito: o resultado, as métricas, as tabelas principais, os gráficos descritos com os seus números (os arcos do percurso, os valores de cada série, as rotas) e as fontes. O que é grande não é enviado se não for pedido: o passo a passo, as tabelas longas e o modelo linear equivalente (`data.lp_model`). Cada relatório traz um `report_id`, e com `get_report_part` pede-se uma seção, um passo (`step`) ou um dado (`path`, por exemplo `data.lp_model`). `detail` escolhe quanto: `summary` (padrão), `steps` (padrão quando `options.steps` é `true`), `full` ou `json` (o relatório inteiro). `describe_module` devolve o esquema e um único exemplo, do tipo de problema pedido (`problem_type`). Um caixeiro-viajante de 10 nós, que antes devolvia cerca de 200 KB, agora devolve cerca de 3 KB.

**Gráficos como imagens.** Com `"images": true`, `solve`, `compare_methods` e `get_report_part` devolvem também os gráficos em PNG (redes, percursos, gráficos, diagramas de Gantt, mapas de rotas, plantas de armazém e árvores), cada um com um rótulo que diz de que parte do relatório vem. Com `"options": {"steps": true}` são incluídos os do passo a passo: no caixeiro-viajante, o percurso parcial depois de cada iteração do vizinho mais próximo (NNH) ou da inserção mais barata (CIH). `max_images` limita quantos (12 por padrão). A fonte tipográfica vem dentro do programa, então as imagens saem iguais em qualquer computador.

**Comparar métodos.** `compare_methods` resolve o mesmo modelo com cada valor da opção de método do tipo de problema e devolve uma tabela com todas as métricas, a melhor (e os empates), a diferença de cada uma em relação à melhor, um gráfico de barras e a solução resumida de cada variante. Funciona em todos os módulos que têm vários métodos: caixeiro-viajante (NNH, CIH, 2-opt, ótimo), soluções iniciais do transporte, árvore geradora mínima, roteirização de veículos, tamanho de lote, previsões, planejamento agregado, programação de tarefas, arranjo físico e gestão de estoques. Com `vary` e `values` escolhe-se qual opção muda e quais valores; com `variants`, combinações livres de opções; com `metric`, a métrica que decide (por exemplo `MSE` em vez de `MAD`). No aplicativo é o botão **Comparar**, que aparece quando o tipo de problema tem métodos para comparar.

## Idiomas

Todo o aplicativo está em **espanhol, inglês e português do Brasil**. O idioma é escolhido em **Configurações** (Ctrl+,) e muda a interface, os relatórios, os exemplos, as fontes (APA 7 em inglês: «&», «10th ed.», «n.d.»; em português: «&» na lista de referências, «e» nas citações narrativas, «10. ed.», «s.d.»; os títulos ficam no seu idioma original) e os gráficos baixados. A mudança é imediata: a interface muda primeiro e o problema aberto é resolvido de novo em segundo plano (com um aviso enquanto isso); os catálogos do núcleo são preparados ao abrir o aplicativo. Na primeira vez, usa-se o idioma do sistema.

- Interface: os textos são escritos em espanhol e passam por `t()` (`apps/desktop/src/lib/i18n.svelte.ts`); o inglês está em `apps/desktop/src/lib/i18n/en_*.ts` e o português em `pt*.ts`.
- Núcleo: `crates/iaqsb-core/src/i18n.rs` traduz o que a API devolve com os catálogos `crates/iaqsb-core/i18n/en.json` e `pt.json` (cerca de 6.000 textos e modelos de frase extraídos do código e dos dados com `python3 tools/i18n_extraer.py`) e `en_extra.json` e `pt_extra.json` (palavras soltas e frases montadas por partes). Os ids, as chaves e os códigos não são traduzidos, e os resultados numéricos são os mesmos nos três idiomas.
- A linguagem algébrica aceita palavras-chave nos três idiomas (`minimizar`/`minimize`/`minimizar`, `sujeto a`/`subject to`/`sujeito a`, `suma`/`sum`/`soma`…).
- CLI: `--lang en`, `--lang pt` ou `IAQSB_LANG`. MCP: o argumento `language: "en"` ou `"pt"` em qualquer ferramenta (ou `IAQSB_LANG` na configuração do servidor).
- O teste `crates/iaqsb-api/tests/todo/idioma.rs` resolve os exemplos em inglês e verifica que saem em inglês, com os mesmos números e com a sua fonte; `crates/iaqsb-core/src/i18n.rs` testa o português.

Para acrescentar outro idioma (por exemplo, francês para o Haiti ou o Caribe), veja [CONTRIBUTING.md](CONTRIBUTING.md).

## Linha de comando

```powershell
iaqsb modulos
iaqsb resolver problema.LPP --pasos        # quadros do simplex
iaqsb resolver QUEUE1.QAA                  # qualquer arquivo do WinQSB
iaqsb resolver tsp.json --pasos --graficas figuras   # salva os gráficos em PNG (--svg: também em SVG)
iaqsb comparar tsp.json                    # todos os métodos do tipo de problema
iaqsb --md resolver tsp.json               # resumo breve em Markdown, para agentes de IA
iaqsb comparar pronostico.json --metrica MSE --valores single_exponential,linear_trend --graficas figuras
iaqsb importar C_CHART.QCC > modelo.json
iaqsb exportar modelo.json -s copia.QCC
iaqsb --json llamar solve < peticion.json  # API completa, para agentes
iaqsb --lang en resolver tsp.json         # em inglês (também: variável IAQSB_LANG=en); --lang pt: português
iaqsb resolver transporte.iaqsb            # modelo da linguagem algébrica
iaqsb revisar transporte.iaqsb             # só verifica: erros com linha e coluna, e tamanho
iaqsb informe wyndor.LPP -s informe.docx --pasos --autor "Grupo 8" --curso "Taller de Producción y Logística"   # .docx, .pdf, .html, .xlsx ou .md
iaqsb convertir wyndor.LPP wyndor.xlsx     # para Excel (e de volta); também .iaqsb, .mps, .lp, .json
iaqsb barrer wyndor.LPP --dato constraints.2.rhs --desde 10 --hasta 30 --pasos 10   # e se…?
iaqsb parametrico wyndor.LPP --lado Planta3   # objetivo segundo o lado direito de Planta3, por partes
iaqsb ajustar modelo.mps                   # ajuste automático dos parâmetros do solver
```

Os módulos novos se chamam `vrp`, `modo`, `inv`, `fin`, `alm` e `kpi`; os seus arquivos (`.VRP`, `.MOD`, `.INV`, `.FIN`, `.ALM`, `.KPI`) são do IAQSB, não do WinQSB.

As operações de `llamar` são `list_modules`, `describe_module`, `spec`, `template`, `solve`, `compare`, `comparable`, `render` (um gráfico do relatório em SVG ou PNG), `graphic`, `import_text`, `export_text`, `examples`, `bibliography`, `translate`, `solver_info`, `diagnose`, `bibtex`, `digest`, `convert`, `check_model`, `sweep`, `parametric`, `tune`, `export_report`, `export_xlsx`, `import_xlsx` e `parse_pasted`; todas recebem e devolvem JSON e, com `"lang": "en"` ou `"pt"`, respondem nesse idioma.

## Modelos

Cada módulo descreve o seu modelo com um esquema JSON (`iaqsb describir <módulo>`) e traz exemplos (`iaqsb --json llamar examples`). Por exemplo, programação linear:

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

Os limites infinitos são escritos `"M"` e `"-M"`, como no WinQSB. Um modelo como esse, escrito na linguagem algébrica:

```text
maximizar z: 3 X1 + 5 X2;
sujeto a
C1: X1 <= 4;
C2: 3 X1 + 2 X2 <= 18;
```

(no modo rápido, sem `var`, as variáveis são declaradas automaticamente e são não negativas; veja [docs/LENGUAJE.md](docs/LENGUAJE.md)).

## Como citar

Se você usar o IAQSB em um trabalho acadêmico, cite a versão que usou (os dados estão em [CITATION.cff](CITATION.cff) e em **Configurações → Sobre o IAQSB**, que também a copia em BibTeX):

IA Nativa. (2026). *IAQSB: investigación de operaciones nativa para IA* (Versão 0.2.0) [Software].

Cite também os métodos e os dados: cada relatório traz as suas referências em APA 7.

## Licença e créditos

Código sob a licença MIT: pode ser usado, estudado, modificado e redistribuído, inclusive em disciplinas, laboratórios de informática e projetos de pesquisa. As contribuições são bem-vindas ([CONTRIBUTING.md](CONTRIBUTING.md)); a comunidade segue o [código de conduta](CODE_OF_CONDUCT.md). O IAQSB é uma reimplementação independente do WinQSB, cujos direitos pertencem ao seu autor, Yih-Long Chang. O código do IAQSB não contém código do WinQSB.

A pasta `crates/iaqsb-api/tests/winqsb` guarda os 64 arquivos de exemplo da distribuição do WinQSB 2.0, que só são usados nos testes de compatibilidade; se você for publicar o código, verifique se a sua permissão os cobre ou apague essa pasta (o teste pode ser removido sem afetar mais nada). Certifique-se de que a sua permissão cobre o uso que você quer dar ao IAQSB.

Os gráficos que o programa desenha (servidor MCP, linha de comando e downloads do aplicativo) usam as fontes tipográficas Inter (licença SIL Open Font 1.1) e DejaVu Sans (licença Bitstream Vera), incluídas com as suas licenças em `crates/iaqsb-render/fuentes`.
