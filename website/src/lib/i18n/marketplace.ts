export const SUPPORTED_LOCALES = [
  'en',
  'zh-cn',
  'zh-tw',
  'ja',
  'ko',
  'ru',
  'fr',
  'pt-br',
  'es',
  'it',
  'de',
  'pl',
] as const;

export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

export interface MarketplaceStrings {
  pageTitle: string;
  pageDescription: string;
  intro: string;
  byAuthor: (author: string) => string;
  installGlobal: string;
  installProject: string;
  submitTitle: string;
  submitDescription: string;
  copyTooltip: string;
  rules: (n: number) => string;
  skills: (n: number) => string;
}

const en: MarketplaceStrings = {
  pageTitle: 'Sets Marketplace',
  pageDescription: 'Browse and install community Claude Code configuration sets',
  intro: 'Community-contributed configuration sets for Claude Code. Each set bundles CLAUDE.md, rules, skills, and MCP server configs into a single installable package.',
  byAuthor: (author) => `by ${author}`,
  installGlobal: 'Global',
  installProject: 'Project',
  submitTitle: 'Submit Your Set',
  submitDescription:
    'Create a <code>.claudex-sets.json</code> in your repo (<a href="https://claudex.space/schemas/sets/v1.json" target="_blank" rel="noopener">schema</a>), then submit a PR to <a href="https://github.com/StringKe/claudex" target="_blank" rel="noopener">claudex</a> adding your entry to <code>sets.json</code>.',
  copyTooltip: 'Copy',
  rules: (n) => `${n} rules`,
  skills: (n) => `${n} skills`,
};

const zhCN: MarketplaceStrings = {
  pageTitle: '配置集市场',
  pageDescription: '浏览并安装社区 Claude Code 配置集',
  intro: '社区贡献的 Claude Code 配置集。每个配置集将 CLAUDE.md、rules、skills 和 MCP 服务器配置打包为一个可安装的包。',
  byAuthor: (author) => `by ${author}`,
  installGlobal: 'Global',
  installProject: 'Project',
  submitTitle: '提交你的配置集',
  submitDescription:
    '在你的仓库根目录创建 <code>.claudex-sets.json</code>（<a href="https://claudex.space/schemas/sets/v1.json" target="_blank" rel="noopener">schema</a>），然后向 <a href="https://github.com/StringKe/claudex" target="_blank" rel="noopener">claudex</a> 提交 PR，将你的条目添加到 <code>sets.json</code>。',
  copyTooltip: '复制',
  rules: (n) => `${n} 条规则`,
  skills: (n) => `${n} 个技能`,
};

const zhTW: MarketplaceStrings = {
  pageTitle: '配置集市場',
  pageDescription: '瀏覽並安裝社區 Claude Code 配置集',
  intro: '社區貢獻的 Claude Code 配置集。每個配置集將 CLAUDE.md、rules、skills 和 MCP 伺服器配置打包為一個可安裝的套件。',
  byAuthor: (author) => `by ${author}`,
  installGlobal: 'Global',
  installProject: 'Project',
  submitTitle: '提交你的配置集',
  submitDescription:
    '在你的儲存庫根目錄建立 <code>.claudex-sets.json</code>（<a href="https://claudex.space/schemas/sets/v1.json" target="_blank" rel="noopener">schema</a>），然後向 <a href="https://github.com/StringKe/claudex" target="_blank" rel="noopener">claudex</a> 提交 PR，將你的條目加入 <code>sets.json</code>。',
  copyTooltip: '複製',
  rules: (n) => `${n} 條規則`,
  skills: (n) => `${n} 個技能`,
};

const ja: MarketplaceStrings = {
  pageTitle: 'セットマーケットプレイス',
  pageDescription: 'コミュニティの Claude Code 設定セットを閲覧・インストール',
  intro: 'コミュニティが提供する Claude Code 設定セット。各セットは CLAUDE.md、rules、skills、MCP サーバー設定を一つのインストール可能なパッケージにまとめています。',
  byAuthor: (author) => `作者: ${author}`,
  installGlobal: 'グローバル',
  installProject: 'プロジェクト',
  submitTitle: 'セットを投稿する',
  submitDescription:
    'リポジトリに <code>.claudex-sets.json</code> を作成し（<a href="https://claudex.space/schemas/sets/v1.json" target="_blank" rel="noopener">schema</a>）、<a href="https://github.com/StringKe/claudex" target="_blank" rel="noopener">claudex</a> に PR を送信して <code>sets.json</code> にエントリを追加してください。',
  copyTooltip: 'コピー',
  rules: (n) => `${n} ルール`,
  skills: (n) => `${n} スキル`,
};

const ko: MarketplaceStrings = {
  pageTitle: '세트 마켓플레이스',
  pageDescription: '커뮤니티 Claude Code 설정 세트 탐색 및 설치',
  intro: '커뮤니티가 기여한 Claude Code 설정 세트입니다. 각 세트는 CLAUDE.md, rules, skills, MCP 서버 설정을 하나의 설치 가능한 패키지로 묶어 제공합니다.',
  byAuthor: (author) => `작성자: ${author}`,
  installGlobal: '전역',
  installProject: '프로젝트',
  submitTitle: '세트 제출하기',
  submitDescription:
    '리포지토리에 <code>.claudex-sets.json</code>을 만들고（<a href="https://claudex.space/schemas/sets/v1.json" target="_blank" rel="noopener">schema</a>）, <a href="https://github.com/StringKe/claudex" target="_blank" rel="noopener">claudex</a>에 PR을 제출하여 <code>sets.json</code>에 항목을 추가하세요.',
  copyTooltip: '복사',
  rules: (n) => `${n}개 규칙`,
  skills: (n) => `${n}개 스킬`,
};

const ru: MarketplaceStrings = {
  pageTitle: 'Маркетплейс наборов',
  pageDescription: 'Просмотр и установка наборов конфигурации Claude Code от сообщества',
  intro: 'Наборы конфигурации Claude Code от сообщества. Каждый набор объединяет CLAUDE.md, rules, skills и конфигурации MCP-серверов в один устанавливаемый пакет.',
  byAuthor: (author) => `автор: ${author}`,
  installGlobal: 'Глобально',
  installProject: 'Проект',
  submitTitle: 'Отправить свой набор',
  submitDescription:
    'Создайте <code>.claudex-sets.json</code> в вашем репозитории (<a href="https://claudex.space/schemas/sets/v1.json" target="_blank" rel="noopener">schema</a>), затем отправьте PR в <a href="https://github.com/StringKe/claudex" target="_blank" rel="noopener">claudex</a>, добавив запись в <code>sets.json</code>.',
  copyTooltip: 'Копировать',
  rules: (n) => `${n} правил`,
  skills: (n) => `${n} навыков`,
};

const fr: MarketplaceStrings = {
  pageTitle: 'Marketplace des sets',
  pageDescription: 'Parcourir et installer les sets de configuration Claude Code de la communaute',
  intro: 'Sets de configuration Claude Code contribues par la communaute. Chaque set regroupe CLAUDE.md, rules, skills et configurations de serveurs MCP dans un seul package installable.',
  byAuthor: (author) => `par ${author}`,
  installGlobal: 'Global',
  installProject: 'Projet',
  submitTitle: 'Soumettre votre set',
  submitDescription:
    'Creez un <code>.claudex-sets.json</code> dans votre depot (<a href="https://claudex.space/schemas/sets/v1.json" target="_blank" rel="noopener">schema</a>), puis soumettez une PR a <a href="https://github.com/StringKe/claudex" target="_blank" rel="noopener">claudex</a> en ajoutant votre entree dans <code>sets.json</code>.',
  copyTooltip: 'Copier',
  rules: (n) => `${n} regles`,
  skills: (n) => `${n} competences`,
};

const ptBR: MarketplaceStrings = {
  pageTitle: 'Marketplace de sets',
  pageDescription: 'Navegue e instale sets de configuracao Claude Code da comunidade',
  intro: 'Sets de configuracao Claude Code contribuidos pela comunidade. Cada set agrupa CLAUDE.md, rules, skills e configuracoes de servidores MCP em um unico pacote instalavel.',
  byAuthor: (author) => `por ${author}`,
  installGlobal: 'Global',
  installProject: 'Projeto',
  submitTitle: 'Envie seu set',
  submitDescription:
    'Crie um <code>.claudex-sets.json</code> no seu repositorio (<a href="https://claudex.space/schemas/sets/v1.json" target="_blank" rel="noopener">schema</a>), depois envie um PR para <a href="https://github.com/StringKe/claudex" target="_blank" rel="noopener">claudex</a> adicionando sua entrada ao <code>sets.json</code>.',
  copyTooltip: 'Copiar',
  rules: (n) => `${n} regras`,
  skills: (n) => `${n} habilidades`,
};

const es: MarketplaceStrings = {
  pageTitle: 'Marketplace de sets',
  pageDescription: 'Explora e instala sets de configuracion de Claude Code de la comunidad',
  intro: 'Sets de configuracion de Claude Code contribuidos por la comunidad. Cada set agrupa CLAUDE.md, rules, skills y configuraciones de servidores MCP en un solo paquete instalable.',
  byAuthor: (author) => `por ${author}`,
  installGlobal: 'Global',
  installProject: 'Proyecto',
  submitTitle: 'Enviar tu set',
  submitDescription:
    'Crea un <code>.claudex-sets.json</code> en tu repositorio (<a href="https://claudex.space/schemas/sets/v1.json" target="_blank" rel="noopener">schema</a>), luego envia un PR a <a href="https://github.com/StringKe/claudex" target="_blank" rel="noopener">claudex</a> agregando tu entrada a <code>sets.json</code>.',
  copyTooltip: 'Copiar',
  rules: (n) => `${n} reglas`,
  skills: (n) => `${n} habilidades`,
};

const it: MarketplaceStrings = {
  pageTitle: 'Marketplace dei set',
  pageDescription: 'Esplora e installa set di configurazione Claude Code della comunita',
  intro: 'Set di configurazione Claude Code contribuiti dalla comunita. Ogni set raggruppa CLAUDE.md, rules, skills e configurazioni di server MCP in un unico pacchetto installabile.',
  byAuthor: (author) => `di ${author}`,
  installGlobal: 'Globale',
  installProject: 'Progetto',
  submitTitle: 'Invia il tuo set',
  submitDescription:
    'Crea un <code>.claudex-sets.json</code> nel tuo repository (<a href="https://claudex.space/schemas/sets/v1.json" target="_blank" rel="noopener">schema</a>), poi invia una PR a <a href="https://github.com/StringKe/claudex" target="_blank" rel="noopener">claudex</a> aggiungendo la tua voce a <code>sets.json</code>.',
  copyTooltip: 'Copia',
  rules: (n) => `${n} regole`,
  skills: (n) => `${n} competenze`,
};

const de: MarketplaceStrings = {
  pageTitle: 'Sets-Marktplatz',
  pageDescription: 'Claude Code Konfigurationssets der Community durchsuchen und installieren',
  intro: 'Von der Community beigesteuerte Konfigurationssets fur Claude Code. Jedes Set bundelt CLAUDE.md, Rules, Skills und MCP-Serverkonfigurationen in ein installierbares Paket.',
  byAuthor: (author) => `von ${author}`,
  installGlobal: 'Global',
  installProject: 'Projekt',
  submitTitle: 'Set einreichen',
  submitDescription:
    'Erstelle eine <code>.claudex-sets.json</code> in deinem Repository (<a href="https://claudex.space/schemas/sets/v1.json" target="_blank" rel="noopener">Schema</a>), dann sende einen PR an <a href="https://github.com/StringKe/claudex" target="_blank" rel="noopener">claudex</a> und fuge deinen Eintrag zu <code>sets.json</code> hinzu.',
  copyTooltip: 'Kopieren',
  rules: (n) => `${n} Regeln`,
  skills: (n) => `${n} Skills`,
};

const pl: MarketplaceStrings = {
  pageTitle: 'Marketplace zestawow',
  pageDescription: 'Przegladaj i instaluj zestawy konfiguracji Claude Code od spolecznosci',
  intro: 'Zestawy konfiguracji Claude Code od spolecznosci. Kazdy zestaw laczy CLAUDE.md, rules, skills i konfiguracje serwerow MCP w jeden instalowalny pakiet.',
  byAuthor: (author) => `autor: ${author}`,
  installGlobal: 'Globalnie',
  installProject: 'Projekt',
  submitTitle: 'Zglos swoj zestaw',
  submitDescription:
    'Utworz <code>.claudex-sets.json</code> w swoim repozytorium (<a href="https://claudex.space/schemas/sets/v1.json" target="_blank" rel="noopener">schema</a>), nastepnie wyslij PR do <a href="https://github.com/StringKe/claudex" target="_blank" rel="noopener">claudex</a> dodajac swoj wpis do <code>sets.json</code>.',
  copyTooltip: 'Kopiuj',
  rules: (n) => `${n} regul`,
  skills: (n) => `${n} umiejetnosci`,
};

const strings: Record<SupportedLocale, MarketplaceStrings> = {
  en,
  'zh-cn': zhCN,
  'zh-tw': zhTW,
  ja,
  ko,
  ru,
  fr,
  'pt-br': ptBR,
  es,
  it,
  de,
  pl,
};

export function getMarketplaceStrings(locale: string): MarketplaceStrings {
  return strings[locale as SupportedLocale] ?? strings.en;
}
