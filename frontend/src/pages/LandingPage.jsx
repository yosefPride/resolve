import Hero from '../components/marketing/Hero';
import IssueDetailSpotlight from '../components/marketing/spotlights/IssueDetailSpotlight';
import AiSpotlight from '../components/marketing/spotlights/AiSpotlight';
import DashboardSpotlight from '../components/marketing/spotlights/DashboardSpotlight';
import WorkflowTimeline from '../components/marketing/WorkflowTimeline';
import FinalCta from '../components/marketing/FinalCta';

export default function LandingPage() {
  return (
    <>
      <Hero />
      <IssueDetailSpotlight />
      <AiSpotlight />
      <DashboardSpotlight />
      <WorkflowTimeline />
      <FinalCta />
    </>
  );
}
