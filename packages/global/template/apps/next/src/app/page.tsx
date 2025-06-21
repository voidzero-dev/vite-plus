import * as console from "@repo/logger";

export const metadata = {
  title: "My Page"
};

export default function MyPage() {
  console.info("Next.js");
  return <div>content</div>;
}
