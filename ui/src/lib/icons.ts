// Les icônes de l'interface : Lucide (licence ISC), livrées avec Cahutin, donc les mêmes sur Windows, macOS et
// Linux. Un nom dit à quoi sert l'icône dans Cahutin ; plusieurs noms peuvent donner le même dessin.
// Pour en ajouter une : https://lucide.dev/icons, puis une ligne ici.
import type { Component } from "svelte";
import ALargeSmall from "@lucide/svelte/icons/a-large-small";
import AlignHorizontalDistributeCenter from "@lucide/svelte/icons/align-horizontal-distribute-center";
import AlignLeft from "@lucide/svelte/icons/align-left";
import ArrowLeft from "@lucide/svelte/icons/arrow-left";
import ArrowRight from "@lucide/svelte/icons/arrow-right";
import Baby from "@lucide/svelte/icons/baby";
import Ban from "@lucide/svelte/icons/ban";
import Bird from "@lucide/svelte/icons/bird";
import BookOpen from "@lucide/svelte/icons/book-open";
import BookOpenText from "@lucide/svelte/icons/book-open-text";
import Braces from "@lucide/svelte/icons/braces";
import Camera from "@lucide/svelte/icons/camera";
import Captions from "@lucide/svelte/icons/captions";
import Cat from "@lucide/svelte/icons/cat";
import Check from "@lucide/svelte/icons/check";
import ChevronDown from "@lucide/svelte/icons/chevron-down";
import ChevronLeft from "@lucide/svelte/icons/chevron-left";
import ChevronRight from "@lucide/svelte/icons/chevron-right";
import ChevronUp from "@lucide/svelte/icons/chevron-up";
import CircleCheck from "@lucide/svelte/icons/circle-check";
import CircleX from "@lucide/svelte/icons/circle-x";
import ClipboardPaste from "@lucide/svelte/icons/clipboard-paste";
import Copy from "@lucide/svelte/icons/copy";
import Crop from "@lucide/svelte/icons/crop";
import Download from "@lucide/svelte/icons/download";
import Ellipsis from "@lucide/svelte/icons/ellipsis";
import Eraser from "@lucide/svelte/icons/eraser";
import Eye from "@lucide/svelte/icons/eye";
import EyeOff from "@lucide/svelte/icons/eye-off";
import File from "@lucide/svelte/icons/file";
import FileInput from "@lucide/svelte/icons/file-input";
import FilePlus from "@lucide/svelte/icons/file-plus";
import FileText from "@lucide/svelte/icons/file-text";
import Flag from "@lucide/svelte/icons/flag";
import FlipHorizontal2 from "@lucide/svelte/icons/flip-horizontal-2";
import FlipVertical2 from "@lucide/svelte/icons/flip-vertical-2";
import Flower2 from "@lucide/svelte/icons/flower-2";
import Folder from "@lucide/svelte/icons/folder";
import FolderOpen from "@lucide/svelte/icons/folder-open";
import Folders from "@lucide/svelte/icons/folders";
import Frame from "@lucide/svelte/icons/frame";
import Grid3x3 from "@lucide/svelte/icons/grid-3x3";
import History from "@lucide/svelte/icons/history";
import House from "@lucide/svelte/icons/house";
import Image from "@lucide/svelte/icons/image";
import Images from "@lucide/svelte/icons/images";
import Layers2 from "@lucide/svelte/icons/layers-2";
import LayoutGrid from "@lucide/svelte/icons/layout-grid";
import LayoutPanelTop from "@lucide/svelte/icons/layout-panel-top";
import LayoutTemplate from "@lucide/svelte/icons/layout-template";
import Lightbulb from "@lucide/svelte/icons/lightbulb";
import Lock from "@lucide/svelte/icons/lock";
import LockOpen from "@lucide/svelte/icons/lock-open";
import Merge from "@lucide/svelte/icons/merge";
import Minus from "@lucide/svelte/icons/minus";
import Move from "@lucide/svelte/icons/move";
import PaintBucket from "@lucide/svelte/icons/paint-bucket";
import Paintbrush from "@lucide/svelte/icons/paintbrush";
import Palette from "@lucide/svelte/icons/palette";
import Pencil from "@lucide/svelte/icons/pencil";
import PencilLine from "@lucide/svelte/icons/pencil-line";
import Pin from "@lucide/svelte/icons/pin";
import Play from "@lucide/svelte/icons/play";
import Plus from "@lucide/svelte/icons/plus";
import Printer from "@lucide/svelte/icons/printer";
import RectangleHorizontal from "@lucide/svelte/icons/rectangle-horizontal";
import RectangleVertical from "@lucide/svelte/icons/rectangle-vertical";
import Redo2 from "@lucide/svelte/icons/redo-2";
import RefreshCw from "@lucide/svelte/icons/refresh-cw";
import Replace from "@lucide/svelte/icons/replace";
import RotateCcw from "@lucide/svelte/icons/rotate-ccw";
import RotateCw from "@lucide/svelte/icons/rotate-cw";
import Ruler from "@lucide/svelte/icons/ruler";
import Save from "@lucide/svelte/icons/save";
import ScanFace from "@lucide/svelte/icons/scan-face";
import Scissors from "@lucide/svelte/icons/scissors";
import Settings from "@lucide/svelte/icons/settings";
import Shapes from "@lucide/svelte/icons/shapes";
import Smile from "@lucide/svelte/icons/smile";
import Sparkles from "@lucide/svelte/icons/sparkles";
import Square from "@lucide/svelte/icons/square";
import SquareRoundCorner from "@lucide/svelte/icons/square-round-corner";
import Star from "@lucide/svelte/icons/star";
import Tag from "@lucide/svelte/icons/tag";
import Trash2 from "@lucide/svelte/icons/trash-2";
import TriangleAlert from "@lucide/svelte/icons/triangle-alert";
import Type from "@lucide/svelte/icons/type";
import Undo2 from "@lucide/svelte/icons/undo-2";
import Users from "@lucide/svelte/icons/users";
import X from "@lucide/svelte/icons/x";
import ZoomIn from "@lucide/svelte/icons/zoom-in";
import ZoomOut from "@lucide/svelte/icons/zoom-out";

export const ICONS = {
  add: Plus,
  alert: TriangleAlert,
  align: AlignLeft,
  arrowLeft: ArrowLeft,
  arrowRight: ArrowRight,
  assistant: Sparkles,
  back: ArrowLeft,
  ban: Ban,
  blur: ScanFace,
  book: BookOpen,
  border: Square,
  braces: Braces,
  camera: Camera,
  caption: Captions,
  check: Check,
  chevronDown: ChevronDown,
  chevronLeft: ChevronLeft,
  chevronRight: ChevronRight,
  chevronUp: ChevronUp,
  child: Baby,
  class: Users,
  clipboard: ClipboardPaste,
  close: X,
  collection: Folders,
  colour: Palette,
  copy: Copy,
  corners: SquareRoundCorner,
  crop: Crop,
  cut: Scissors,
  distribute: AlignHorizontalDistributeCenter,
  document: File,
  documentText: FileText,
  duplicate: Copy,
  edit: Pencil,
  empty: Eraser,
  example: Sparkles,
  file: File,
  fileImport: FileInput,
  fileNew: FilePlus,
  fill: PaintBucket,
  flag: Flag,
  flipH: FlipHorizontal2,
  flipV: FlipVertical2,
  folder: Folder,
  folderOpen: FolderOpen,
  frame: Frame,
  free: Move,
  frieze: Flower2,
  grid: LayoutGrid,
  guide: BookOpenText,
  history: History,
  home: House,
  image: Image,
  images: Images,
  import: Download,
  landscape: RectangleHorizontal,
  layers: Layers2,
  lightbulb: Lightbulb,
  lock: Lock,
  maze: Bird,
  merge: Merge,
  minus: Minus,
  more: Ellipsis,
  move: Move,
  page: File,
  paint: Paintbrush,
  pen: PencilLine,
  photo: Image,
  picto: Smile,
  pin: Pin,
  play: Play,
  portrait: RectangleVertical,
  print: Printer,
  redo: Redo2,
  refresh: RefreshCw,
  replace: Replace,
  restart: RotateCcw,
  rotateLeft: RotateCcw,
  rotateRight: RotateCw,
  ruler: Ruler,
  save: Save,
  settings: Settings,
  shadow: Layers2,
  shapes: Shapes,
  spread: BookOpen,
  star: Star,
  symbol: Cat,
  tag: Tag,
  template: LayoutTemplate,
  text: Type,
  trash: Trash2,
  undo: Undo2,
  unlock: LockOpen,
  users: Users,
  warning: TriangleAlert,
  x: X,
  zoomIn: ZoomIn,
  zoomOut: ZoomOut,
  eye: Eye,
  eyeOff: EyeOff,
  circleCheck: CircleCheck,
  circleX: CircleX,
  fonts: ALargeSmall,
  decor: Flower2,
  repeat: Grid3x3,
  background: LayoutPanelTop,
  cvd: Eye,
} satisfies Record<string, Component<any>>;

export type IconName = keyof typeof ICONS;
